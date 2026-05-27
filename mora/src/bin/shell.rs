use mishell_parser::{Parser, Command, CommandBody, SimpleCommand, Pipeline, Word, WordPart, Redirect, RedirectOp, RedirectTarget};
use std::collections::HashMap;
use std::process::{Command as ProcessCommand, Stdio, Child};
use std::io::{self, Write, Read};
use std::path::PathBuf;
use anyhow::{anyhow, Result};

pub struct Shell {
    vars: HashMap<String, String>,
    aliases: HashMap<String, String>,
    abbreviations: HashMap<String, String>,
    fish_features: bool,
    last_exit_code: i32,
    dir_stack: Vec<PathBuf>,
}

impl Shell {
    pub fn new(fish_features: bool) -> Result<Self> {
        let mut vars = HashMap::new();

        // Import environment variables
        for (key, value) in std::env::vars() {
            vars.insert(key, value);
        }

        // Set defaults
        vars.entry("PS1".to_string()).or_insert_with(|| "\\u@\\h:\\w$ ".to_string());
        vars.entry("PROMPT".to_string()).or_insert_with(|| "mishell> ".to_string());

        Ok(Self {
            vars,
            aliases: HashMap::new(),
            abbreviations: HashMap::new(),
            fish_features,
            last_exit_code: 0,
            dir_stack: Vec::new(),
        })
    }

    pub fn execute(&mut self, input: &str) -> Result<()> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(());
        }

        // Expand abbreviations (fish feature)
        let input = if self.fish_features {
            self.expand_abbreviations(input)
        } else {
            input.to_string()
        };

        // Parse
        let mut parser = Parser::new(&input);
        let commands = parser.parse()?;

        for cmd in commands {
            self.execute_command(&cmd)?;
        }

        Ok(())
    }

    fn expand_abbreviations(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (abbr, expansion) in &self.abbreviations {
            // Only expand at word boundaries
            let pattern = format!(" {} ", abbr);
            let replacement = format!(" {} ", expansion);
            result = result.replace(&pattern, &replacement);

            // Also expand at start
            if result.starts_with(abbr) {
                result = format!("{}{}", expansion, &result[abbr.len()..]);
            }
        }
        result
    }

    fn execute_command(&mut self, cmd: &Command) -> Result<()> {
        match &cmd.body {
            CommandBody::Simple(simple) => {
                self.execute_simple(simple, cmd.background)?;
            }
            CommandBody::Pipeline(pipeline) => {
                self.execute_pipeline(pipeline)?;
            }
            CommandBody::And(left, right) => {
                self.execute_command_body(left)?;
                if self.last_exit_code == 0 {
                    self.execute_command_body(right)?;
                }
            }
            CommandBody::Or(left, right) => {
                self.execute_command_body(left)?;
                if self.last_exit_code != 0 {
                    self.execute_command_body(right)?;
                }
            }
            CommandBody::Sequence(left, right) => {
                self.execute_command_body(left)?;
                self.execute_command_body(right)?;
            }
            CommandBody::Subshell(body) => {
                // Execute in subshell (simplified - just execute)
                self.execute_command_body(body)?;
            }
            CommandBody::Group(body) => {
                self.execute_command_body(body)?;
            }
        }
        Ok(())
    }

    fn execute_command_body(&mut self, body: &CommandBody) -> Result<()> {
        let cmd = Command {
            body: body.clone(),
            background: false,
        };
        self.execute_command(&cmd)
    }

    fn execute_simple(&mut self, cmd: &SimpleCommand, background: bool) -> Result<()> {
        if cmd.words.is_empty() {
            return Ok(());
        }

        // Check for assignments
        for assign in &cmd.assignments {
            let value = self.expand_word(&assign.value);
            self.vars.insert(assign.name.clone(), value);
        }

        if cmd.words.is_empty() {
            return Ok(());
        }

        // Get command name
        let cmd_name = self.expand_word(&cmd.words[0]);

        // Check for builtins
        match cmd_name.as_str() {
            "cd" => return self.builtin_cd(&cmd.words[1..]),
            "export" => return self.builtin_export(&cmd.words[1..]),
            "alias" => return self.builtin_alias(&cmd.words[1..]),
            "abbr" => return self.builtin_abbr(&cmd.words[1..]),
            "set" => return self.builtin_set(&cmd.words[1..]),
            "exit" => {
                let code = cmd.words.get(1)
                    .map(|w| self.expand_word(w).parse::<i32>().unwrap_or(0))
                    .unwrap_or(0);
                std::process::exit(code);
            }
            "pushd" => return self.builtin_pushd(&cmd.words[1..]),
            "popd" => return self.builtin_popd(),
            "dirs" => return self.builtin_dirs(),
            "history" => return self.builtin_history(),
            _ => {}
        }

        // Check aliases
        let actual_cmd = if let Some(alias) = self.aliases.get(&cmd_name) {
            alias.clone()
        } else {
            cmd_name.clone()
        };

        // Build process command
        let mut process_cmd = ProcessCommand::new("sh");
        process_cmd.arg("-c");

        // Reconstruct command string
        let mut cmd_str = actual_cmd;
        for word in &cmd.words[1..] {
            cmd_str.push(' ');
            cmd_str.push_str(&self.expand_word(word));
        }

        process_cmd.arg(&cmd_str);

        // Handle redirects
        for redirect in &cmd.redirects {
            match redirect {
                Redirect { fd: None, op: RedirectOp::Output, target: RedirectTarget::File(path) } => {
                    let path = self.expand_word(path);
                    process_cmd.stdout(Stdio::from(std::fs::File::create(path)?));
                }
                Redirect { fd: None, op: RedirectOp::Append, target: RedirectTarget::File(path) } => {
                    let path = self.expand_word(path);
                    process_cmd.stdout(Stdio::from(std::fs::OpenOptions::new().create(true).append(true).open(path)?));
                }
                Redirect { fd: None, op: RedirectOp::Input, target: RedirectTarget::File(path) } => {
                    let path = self.expand_word(path);
                    process_cmd.stdin(Stdio::from(std::fs::File::open(path)?));
                }
                _ => {}
            }
        }

        // Execute
        if background {
            process_cmd.spawn()?;
        } else {
            let status = process_cmd.status()?;
            self.last_exit_code = status.code().unwrap_or(1);
        }

        Ok(())
    }

    fn execute_pipeline(&mut self, pipeline: &Pipeline) -> Result<()> {
        if pipeline.commands.is_empty() {
            return Ok(());
        }

        if pipeline.commands.len() == 1 {
            return self.execute_simple(&pipeline.commands[0], false);
        }

        // Simple pipeline execution using sh
        let mut cmd_str = String::new();
        for (i, cmd) in pipeline.commands.iter().enumerate() {
            if i > 0 {
                cmd_str.push_str(" | ");
            }
            for (j, word) in cmd.words.iter().enumerate() {
                if j > 0 {
                    cmd_str.push(' ');
                }
                cmd_str.push_str(&self.expand_word(word));
            }
        }

        let mut process_cmd = ProcessCommand::new("sh");
        process_cmd.arg("-c").arg(&cmd_str);

        let status = process_cmd.status()?;
        self.last_exit_code = status.code().unwrap_or(1);

        Ok(())
    }

    fn expand_word(&self, word: &Word) -> String {
        let mut result = String::new();
        for part in &word.parts {
            match part {
                WordPart::Literal(s) => result.push_str(s),
                WordPart::Variable(name) => {
                    let value = self.vars.get(name)
                        .cloned()
                        .unwrap_or_else(|| std::env::var(name).unwrap_or_default());
                    result.push_str(&value);
                }
                WordPart::SingleQuoted(s) => result.push_str(s),
                WordPart::DoubleQuoted(parts) => {
                    for p in parts {
                        match p {
                            WordPart::Literal(s) => result.push_str(s),
                            WordPart::Variable(name) => {
                                let value = self.vars.get(name)
                                    .cloned()
                                    .unwrap_or_else(|| std::env::var(name).unwrap_or_default());
                                result.push_str(&value);
                            }
                            _ => {}
                        }
                    }
                }
                WordPart::Tilde(_) => {
                    if let Some(home) = dirs::home_dir() {
                        result.push_str(&home.to_string_lossy());
                    } else {
                        result.push('~');
                    }
                }
                WordPart::Escape(c) => result.push(*c),
                _ => {}
            }
        }
        result
    }

    // Builtins
    fn builtin_cd(&mut self, args: &[Word]) -> Result<()> {
        let path = if args.is_empty() {
            dirs::home_dir().ok_or_else(|| anyhow!("No home directory"))?
        } else {
            let path_str = self.expand_word(&args[0]);
            let path = PathBuf::from(&path_str);
            if path_str == "-" {
                self.dir_stack.last().cloned().unwrap_or_else(|| PathBuf::from("."))
            } else if path_str.starts_with('~') {
                if let Some(home) = dirs::home_dir() {
                    home.join(&path_str[2..])
                } else {
                    path
                }
            } else {
                path
            }
        };

        let old_dir = std::env::current_dir()?;
        self.dir_stack.push(old_dir);

        std::env::set_current_dir(&path)?;
        self.vars.insert("PWD".to_string(), path.to_string_lossy().to_string());
        self.vars.insert("OLDPWD".to_string(), old_dir.to_string_lossy().to_string());

        Ok(())
    }

    fn builtin_export(&mut self, args: &[Word]) -> Result<()> {
        for arg in args {
            let s = self.expand_word(arg);
            if let Some(eq_pos) = s.find('=') {
                let name = s[..eq_pos].to_string();
                let value = s[eq_pos + 1..].to_string();
                std::env::set_var(&name, &value);
                self.vars.insert(name, value);
            }
        }
        Ok(())
    }

    fn builtin_alias(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            for (name, value) in &self.aliases {
                println!("alias {}='{}'", name, value);
            }
            return Ok(());
        }

        for arg in args {
            let s = self.expand_word(arg);
            if let Some(eq_pos) = s.find('=') {
                let name = s[..eq_pos].to_string();
                let value = s[eq_pos + 1..].to_string();
                self.aliases.insert(name, value);
            }
        }
        Ok(())
    }

    fn builtin_abbr(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            for (name, value) in &self.abbreviations {
                println!("abbr --add {} {}", name, value);
            }
            return Ok(());
        }

        // Parse: abbr --add name expansion
        let mut args_iter = args.iter();
        while let Some(arg) = args_iter.next() {
            let s = self.expand_word(arg);
            if s == "--add" || s == "-a" {
                if let (Some(name), Some(expansion)) = (args_iter.next(), args_iter.next()) {
                    let name = self.expand_word(name);
                    let expansion = self.expand_word(expansion);
                    self.abbreviations.insert(name, expansion);
                }
            }
        }
        Ok(())
    }

    fn builtin_set(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            for (key, value) in &self.vars {
                println!("{}={}", key, value);
            }
            return Ok(());
        }

        let mut args_iter = args.iter();
        while let Some(arg) = args_iter.next() {
            let s = self.expand_word(arg);
            if let Some(eq_pos) = s.find('=') {
                let name = s[..eq_pos].to_string();
                let value = s[eq_pos + 1..].to_string();
                self.vars.insert(name, value);
            }
        }
        Ok(())
    }

    fn builtin_pushd(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            // Swap top two
            if self.dir_stack.len() >= 2 {
                let len = self.dir_stack.len();
                self.dir_stack.swap(len - 1, len - 2);
            }
        } else {
            let path = self.expand_word(&args[0]);
            let current = std::env::current_dir()?;
            self.dir_stack.push(current);
            std::env::set_current_dir(&path)?;
        }

        self.builtin_dirs()
    }

    fn builtin_popd(&mut self) -> Result<()> {
        if let Some(dir) = self.dir_stack.pop() {
            std::env::set_current_dir(&dir)?;
            self.vars.insert("PWD".to_string(), dir.to_string_lossy().to_string());
            self.builtin_dirs()
        } else {
            eprintln!("popd: directory stack empty");
            Ok(())
        }
    }

    fn builtin_dirs(&self) -> Result<()> {
        let current = std::env::current_dir()?;
        print!("{}", current.display());
        for dir in self.dir_stack.iter().rev() {
            print!(" {}", dir.display());
        }
        println!();
        Ok(())
    }

    fn builtin_history(&self) -> Result<()> {
        // History is handled by the History struct in the main loop
        println!("history: use up/down arrows to navigate");
        Ok(())
    }

    pub fn get_var(&self, name: &str) -> Option<&String> {
        self.vars.get(name)
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn abbreviations(&self) -> &HashMap<String, String> {
        &self.abbreviations
    }
}
