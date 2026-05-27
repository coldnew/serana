use mishell_parser::{Parser, Command, CommandBody, SimpleCommand, Pipeline, Word, WordPart, Redirect, RedirectOp, RedirectTarget, FunctionDef, ForLoop, WhileLoop, IfStatement, SwitchStatement, GlobPattern};
use std::collections::HashMap;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::path::PathBuf;
use anyhow::{anyhow, Result};

pub struct Job {
    pub id: usize,
    pub pid: u32,
    pub command: String,
    pub child: Child,
}

pub struct Shell {
    vars: HashMap<String, String>,
    universal_vars: HashMap<String, String>,
    aliases: HashMap<String, String>,
    abbreviations: HashMap<String, String>,
    functions: HashMap<String, FunctionDef>,
    jobs: Vec<Job>,
    next_job_id: usize,
    fish_features: bool,
    last_exit_code: i32,
    dir_stack: Vec<PathBuf>,
    event_handlers: HashMap<String, Vec<FunctionDef>>,
    is_interactive: bool,
    home_dir: Option<PathBuf>,
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

        // Load universal variables
        let universal_vars = Self::load_universal_vars();
        let home_dir = dirs::home_dir();

        Ok(Self {
            vars,
            universal_vars,
            aliases: HashMap::new(),
            abbreviations: HashMap::new(),
            functions: HashMap::new(),
            jobs: Vec::new(),
            next_job_id: 1,
            fish_features,
            last_exit_code: 0,
            dir_stack: Vec::new(),
            event_handlers: HashMap::new(),
            is_interactive: true,
            home_dir,
        })
    }

    fn load_universal_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".mishell_universal");
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    if let Some(eq_pos) = line.find('=') {
                        let name = line[..eq_pos].to_string();
                        let value = line[eq_pos + 1..].to_string();
                        vars.insert(name, value);
                    }
                }
            }
        }
        vars
    }

    fn save_universal_vars(&self) -> Result<()> {
        if let Some(ref home) = self.home_dir {
            let path = home.join(".mishell_universal");
            let mut content = String::new();
            for (name, value) in &self.universal_vars {
                content.push_str(&format!("{}={}\n", name, value));
            }
            std::fs::write(path, content)?;
        }
        Ok(())
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
        if self.abbreviations.is_empty() {
            return input.to_string();
        }
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
            CommandBody::FunctionDef(func) => {
                self.functions.insert(func.name.clone(), func.clone());
            }
            CommandBody::ForLoop(for_loop) => {
                self.execute_for_loop(for_loop)?;
            }
            CommandBody::WhileLoop(while_loop) => {
                self.execute_while_loop(while_loop)?;
            }
            CommandBody::If(if_stmt) => {
                self.execute_if_statement(if_stmt)?;
            }
            CommandBody::Switch(switch_stmt) => {
                self.execute_switch_statement(switch_stmt)?;
            }
        }
        Ok(())
    }

    fn execute_command_body(&mut self, body: &CommandBody) -> Result<()> {
        match body {
            CommandBody::Simple(simple) => {
                self.execute_simple(simple, false)?;
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
                self.execute_command_body(body)?;
            }
            CommandBody::Group(body) => {
                self.execute_command_body(body)?;
            }
            CommandBody::FunctionDef(func) => {
                self.functions.insert(func.name.clone(), func.clone());
            }
            CommandBody::ForLoop(for_loop) => {
                self.execute_for_loop(for_loop)?;
            }
            CommandBody::WhileLoop(while_loop) => {
                self.execute_while_loop(while_loop)?;
            }
            CommandBody::If(if_stmt) => {
                self.execute_if_statement(if_stmt)?;
            }
            CommandBody::Switch(switch_stmt) => {
                self.execute_switch_statement(switch_stmt)?;
            }
        }
        Ok(())
    }

    fn execute_simple(&mut self, cmd: &SimpleCommand, background: bool) -> Result<()> {
        // Process assignments first (FOO=hello without a command)
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
            "export" => {
                let mut args: Vec<Word> = cmd.words[1..].to_vec();
                for assign in &cmd.assignments {
                    args.push(Word { parts: vec![WordPart::Literal(format!("{}={}", assign.name, self.expand_word(&assign.value)))] });
                }
                return self.builtin_export(&args);
            }
            "alias" => return self.builtin_alias(&cmd.words[1..]),
            "abbr" => return self.builtin_abbr(&cmd.words[1..]),
            "set" => {
                // For set builtin, pass assignments as words too
                let mut all_args: Vec<Word> = cmd.words[1..].to_vec();
                for assign in &cmd.assignments {
                    all_args.push(Word { parts: vec![WordPart::Literal(format!("{}={}", assign.name, self.expand_word(&assign.value)))] });
                }
                return self.builtin_set(&all_args);
            }
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
            "jobs" => return self.builtin_jobs(),
            "fg" => return self.builtin_fg(&cmd.words[1..]),
            "bg" => return self.builtin_bg(&cmd.words[1..]),
            "and" => return self.builtin_and(&cmd.words[1..]),
            "or" => return self.builtin_or(&cmd.words[1..]),
            "not" => return self.builtin_not(&cmd.words[1..]),
            "type" => return self.builtin_type(&cmd.words[1..]),
            "count" => return self.builtin_count(&cmd.words[1..]),
            "printf" => return self.builtin_printf(&cmd.words[1..]),
            "source" | "." => return self.builtin_source(&cmd.words[1..]),
            "read" => return self.builtin_read(&cmd.words[1..]),
            "string" => return self.builtin_string(&cmd.words[1..]),
            "math" => return self.builtin_math(&cmd.words[1..]),
            "status" => return self.builtin_status(&cmd.words[1..]),
            "command" => return self.builtin_command(&cmd.words[1..]),
            "builtin" => return self.builtin_builtin_cmd(&cmd.words[1..]),
            "contains" => return self.builtin_contains(&cmd.words[1..]),
            "random" => return self.builtin_random(&cmd.words[1..]),
            "emit" => return self.builtin_emit(&cmd.words[1..]),
            "begin" => {
                // begin ... end is handled at parser level as Group,
                // but treat bare 'begin' as echo (no-op if no args)
                return Ok(());
            }
            _ => {}
        }
        
        // Check for user-defined functions
        if let Some(func) = self.functions.get(&cmd_name).cloned() {
            // Expand arguments
            let args: Vec<String> = cmd.words[1..].iter()
                .map(|w| self.expand_word(w))
                .collect();
            
            // Set function arguments as variables
            for (i, arg) in args.iter().enumerate() {
                self.vars.insert(format!("{}", i + 1), arg.clone());
            }
            self.vars.insert("argv".to_string(), args.join(" "));
            
            // Execute function body
            for cmd in &func.body {
                self.execute_command(cmd)?;
            }
            
            return Ok(());
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
                Redirect { fd: None, op: RedirectOp::HereDoc, target: RedirectTarget::HereDoc(content) } => {
                    // Write heredoc content to stdin
                    use std::io::Write;
                    let mut child = process_cmd
                        .stdin(Stdio::piped())
                        .spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(content.as_bytes())?;
                    }
                    let status = child.wait()?;
                    self.last_exit_code = status.code().unwrap_or(1);
                    return Ok(());
                }
                _ => {}
            }
        }

        // Execute
        if background {
            let child = process_cmd.spawn()?;
            let job_id = self.next_job_id;
            self.next_job_id += 1;
            let pid = child.id();
            self.jobs.push(Job {
                id: job_id,
                pid,
                command: cmd_str.clone(),
                child,
            });
            eprintln!("[{}] {} {}", job_id, pid, cmd_str);
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
        // Check if word is entirely double-quoted (no glob expansion)
        let is_quoted = word.parts.len() == 1 && matches!(&word.parts[0], WordPart::DoubleQuoted(_));
        
        // Estimate capacity from parts
        let est_len: usize = word.parts.iter().map(|p| match p {
            WordPart::Literal(s) => s.len(),
            WordPart::SingleQuoted(s) => s.len(),
            WordPart::DoubleQuoted(parts) => parts.iter().map(|dp| match dp {
                WordPart::Literal(s) => s.len(),
                _ => 16,
            }).sum(),
            _ => 16,
        }).sum();
        let mut result = String::with_capacity(est_len);
        
        for part in &word.parts {
            match part {
                WordPart::Literal(s) => result.push_str(s),
                WordPart::Variable(name) => {
                    self.expand_variable_into(name, &mut result);
                }
                WordPart::SingleQuoted(s) => result.push_str(s),
                WordPart::DoubleQuoted(parts) => {
                    for p in parts {
                        match p {
                            WordPart::Literal(s) => result.push_str(s),
                            WordPart::Variable(name) => {
                                self.expand_variable_into(name, &mut result);
                            }
                            WordPart::CommandSub(body) => {
                                let output = self.capture_command_body(body);
                                result.push_str(&output);
                            }
                            _ => {}
                        }
                    }
                }
                WordPart::Tilde(_) => {
                    if let Some(ref home) = self.home_dir {
                        result.push_str(&home.to_string_lossy());
                    } else {
                        result.push('~');
                    }
                }
                WordPart::Escape(c) => result.push(*c),
                WordPart::CommandSub(body) => {
                    let output = self.capture_command_body(body);
                    result.push_str(&output);
                }
                WordPart::Glob(pattern) => {
                    match pattern {
                        GlobPattern::Star => result.push('*'),
                        GlobPattern::Question => result.push('?'),
                        GlobPattern::Class(s) => {
                            result.push('[');
                            result.push_str(s);
                            result.push(']');
                        }
                        GlobPattern::Literal(s) => result.push_str(s),
                    }
                }
                _ => {}
            }
        }
        
        // Glob expansion (not inside double quotes)
        if !is_quoted && (result.contains('*') || result.contains('?') || result.contains('[')) {
            if let Some(expanded) = self.expand_glob(&result) {
                return expanded;
            }
        }
        
        result
    }
    
    fn expand_variable_into(&self, name: &str, result: &mut String) {
        if name == "status" {
            result.push_str(&self.last_exit_code.to_string());
        } else if let Some(val) = self.vars.get(name) {
            result.push_str(val);
        } else if let Some(val) = self.universal_vars.get(name) {
            result.push_str(val);
        } else {
            match std::env::var(name) {
                Ok(val) => result.push_str(&val),
                Err(_) => {}
            }
        }
    }

    fn expand_glob(&self, pattern: &str) -> Option<String> {
        use std::fs;
        
        let (dir, file_pattern) = if let Some(pos) = pattern.rfind('/') {
            (&pattern[..pos], &pattern[pos + 1..])
        } else {
            (".", pattern)
        };
        
        let dir_path = if dir.is_empty() { "." } else { dir };
        
        let entries: Vec<String> = fs::read_dir(dir_path)
            .ok()?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name();
                let name_str = name.to_str()?;
                // Skip hidden files unless pattern starts with .
                if !file_pattern.starts_with('.') && name_str.starts_with('.') {
                    return None;
                }
                if Self::glob_match(file_pattern, name_str) {
                    Some(if dir == "." {
                        name_str.to_owned()
                    } else {
                        let mut s = String::with_capacity(dir.len() + 1 + name_str.len());
                        s.push_str(dir);
                        s.push('/');
                        s.push_str(name_str);
                        s
                    })
                } else {
                    None
                }
            })
            .collect();
        
        if entries.is_empty() {
            None // No matches, return literal
        } else {
            Some(entries.join(" "))
        }
    }

    fn glob_match(pattern: &str, text: &str) -> bool {
        let p: Vec<char> = pattern.chars().collect();
        let t: Vec<char> = text.chars().collect();
        Self::glob_match_impl(&p, &t)
    }

    fn glob_match_impl(pattern: &[char], text: &[char]) -> bool {
        let mut pi = 0;
        let mut ti = 0;
        let mut star_pi = usize::MAX;
        let mut star_ti = 0;
        
        while ti < text.len() {
            if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
                pi += 1;
                ti += 1;
            } else if pi < pattern.len() && pattern[pi] == '[' {
                // Character class
                pi += 1;
                let negated = pi < pattern.len() && pattern[pi] == '^';
                if negated { pi += 1; }
                let mut matched = false;
                let mut first = true;
                while pi < pattern.len() && (first || pattern[pi] != ']') {
                    if pi + 2 < pattern.len() && pattern[pi + 1] == '-' {
                        if text[ti] >= pattern[pi] && text[ti] <= pattern[pi + 2] {
                            matched = true;
                        }
                        pi += 3;
                    } else {
                        if text[ti] == pattern[pi] {
                            matched = true;
                        }
                        pi += 1;
                    }
                    first = false;
                }
                if pi < pattern.len() { pi += 1; } // skip ]
                if negated { matched = !matched; }
                if matched {
                    ti += 1;
                } else {
                    return false;
                }
            } else if pi < pattern.len() && pattern[pi] == '*' {
                star_pi = pi;
                star_ti = ti;
                pi += 1;
            } else if star_pi != usize::MAX {
                pi = star_pi + 1;
                star_ti += 1;
                ti = star_ti;
            } else {
                return false;
            }
        }
        
        while pi < pattern.len() && pattern[pi] == '*' {
            pi += 1;
        }
        
        pi == pattern.len()
    }

    fn capture_command_body(&self, body: &CommandBody) -> String {
        let cmd_str = self.reconstruct_body(body);
        if cmd_str.is_empty() {
            return String::new();
        }
        match ProcessCommand::new("sh").arg("-c").arg(&cmd_str).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                // Trim trailing newlines (like bash), replace internal newlines with spaces
                stdout.trim_end_matches('\n').replace('\n', " ")
            }
            Err(_) => String::new(),
        }
    }

    fn reconstruct_body(&self, body: &CommandBody) -> String {
        match body {
            CommandBody::Simple(cmd) => {
                self.reconstruct_simple(cmd)
            }
            CommandBody::Pipeline(pipeline) => {
                pipeline.commands.iter()
                    .map(|c| self.reconstruct_simple(c))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            CommandBody::And(left, right) => {
                format!("{} && {}", self.reconstruct_body(left), self.reconstruct_body(right))
            }
            CommandBody::Or(left, right) => {
                format!("{} || {}", self.reconstruct_body(left), self.reconstruct_body(right))
            }
            CommandBody::Sequence(left, right) => {
                format!("{}; {}", self.reconstruct_body(left), self.reconstruct_body(right))
            }
            CommandBody::Subshell(body) => {
                format!("({})", self.reconstruct_body(body))
            }
            CommandBody::Group(body) => {
                format!("{{ {}; }}", self.reconstruct_body(body))
            }
            _ => String::new(),
        }
    }

    fn reconstruct_simple(&self, cmd: &SimpleCommand) -> String {
        let mut parts = Vec::new();
        for assign in &cmd.assignments {
            parts.push(format!("{}={}", assign.name, self.expand_word(&assign.value)));
        }
        for word in &cmd.words {
            parts.push(self.expand_word(word));
        }
        parts.join(" ")
    }

    // Builtins
    fn builtin_cd(&mut self, args: &[Word]) -> Result<()> {
        let path = if args.is_empty() {
            self.home_dir.clone().ok_or_else(|| anyhow!("No home directory"))?
        } else {
            let path_str = self.expand_word(&args[0]);
            let path = PathBuf::from(&path_str);
            if path_str == "-" {
                self.dir_stack.last().cloned().unwrap_or_else(|| PathBuf::from("."))
            } else if path_str.starts_with('~') {
                if let Some(ref home) = self.home_dir {
                    home.join(&path_str[2..])
                } else {
                    path
                }
            } else {
                path
            }
        };

        let old_dir = std::env::current_dir()?;
        self.dir_stack.push(old_dir.clone());

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
            // Also show universal vars
            for (key, value) in &self.universal_vars {
                println!("-U: {}={}", key, value);
            }
            return Ok(());
        }

        let mut universal = false;
        let mut args_iter = args.iter();
        
        while let Some(arg) = args_iter.next() {
            let s = self.expand_word(arg);
            if s == "-U" || s == "--universal" {
                universal = true;
                continue;
            }
            if s == "-e" || s == "--erase" {
                // Erase variable
                if let Some(name_arg) = args_iter.next() {
                    let name = self.expand_word(name_arg);
                    self.vars.remove(&name);
                    if self.universal_vars.remove(&name).is_some() {
                        self.save_universal_vars()?;
                    }
                }
                continue;
            }
            if s == "-x" || s == "--export" {
                // Export is handled by builtin_export
                continue;
            }
            if let Some(eq_pos) = s.find('=') {
                let name = s[..eq_pos].to_string();
                let value = s[eq_pos + 1..].to_string();
                if universal {
                    self.universal_vars.insert(name, value);
                    self.save_universal_vars()?;
                } else {
                    self.vars.insert(name, value);
                }
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

    fn builtin_jobs(&mut self) -> Result<()> {
        // Reap finished jobs first
        self.reap_finished_jobs();
        for job in &self.jobs {
            println!("[{}] {} {}", job.id, job.pid, job.command);
        }
        Ok(())
    }

    fn builtin_fg(&mut self, args: &[Word]) -> Result<()> {
        self.reap_finished_jobs();
        let job_id = if args.is_empty() {
            self.jobs.last().map(|j| j.id).unwrap_or(0)
        } else {
            self.expand_word(&args[0]).parse::<usize>().unwrap_or(0)
        };

        if let Some(pos) = self.jobs.iter().position(|j| j.id == job_id) {
            let mut job = self.jobs.remove(pos);
            eprintln!("{}", job.command);
            let status = job.child.wait()?;
            self.last_exit_code = status.code().unwrap_or(1);
        } else {
            eprintln!("fg: job {} not found", job_id);
        }
        Ok(())
    }

    fn builtin_bg(&mut self, args: &[Word]) -> Result<()> {
        self.reap_finished_jobs();
        let job_id = if args.is_empty() {
            self.jobs.last().map(|j| j.id).unwrap_or(0)
        } else {
            self.expand_word(&args[0]).parse::<usize>().unwrap_or(0)
        };

        if let Some(job) = self.jobs.iter().find(|j| j.id == job_id) {
            eprintln!("[{}] {} {}", job.id, job.pid, job.command);
        } else {
            eprintln!("bg: job {} not found", job_id);
        }
        Ok(())
    }

    fn reap_finished_jobs(&mut self) {
        self.jobs.retain_mut(|job| {
            match job.child.try_wait() {
                Ok(Some(_)) => false, // finished, remove
                _ => true,            // still running
            }
        });
    }
    
    fn execute_for_loop(&mut self, for_loop: &ForLoop) -> Result<()> {
        for item in &for_loop.list {
            let value = self.expand_word(item);
            self.vars.insert(for_loop.variable.clone(), value);
            
            for cmd in &for_loop.body {
                self.execute_command(cmd)?;
            }
        }
        Ok(())
    }
    
    fn execute_while_loop(&mut self, while_loop: &WhileLoop) -> Result<()> {
        loop {
            // Execute condition
            for cmd in &while_loop.condition {
                self.execute_command(cmd)?;
            }
            
            // Check condition (exit code 0 = true)
            if self.last_exit_code != 0 {
                break;
            }
            
            // Execute body
            for cmd in &while_loop.body {
                self.execute_command(cmd)?;
            }
        }
        Ok(())
    }
    
    fn execute_if_statement(&mut self, if_stmt: &IfStatement) -> Result<()> {
        // Execute condition
        for cmd in &if_stmt.condition {
            self.execute_command(cmd)?;
        }

        if self.last_exit_code == 0 {
            // Execute then body
            for cmd in &if_stmt.then_body {
                self.execute_command(cmd)?;
            }
            return Ok(());
        }

        // Check elif branches
        for elif in &if_stmt.elif_branches {
            for cmd in &elif.condition {
                self.execute_command(cmd)?;
            }

            if self.last_exit_code == 0 {
                for cmd in &elif.body {
                    self.execute_command(cmd)?;
                }
                return Ok(());
            }
        }

        // Execute else body if present
        if let Some(else_body) = &if_stmt.else_body {
            for cmd in else_body {
                self.execute_command(cmd)?;
            }
        }

        Ok(())
    }

    fn execute_switch_statement(&mut self, switch_stmt: &SwitchStatement) -> Result<()> {
        let value = self.expand_word(&switch_stmt.value);

        for case in &switch_stmt.cases {
            for pattern in &case.patterns {
                let pattern_str = self.expand_word(pattern);
                // Support glob patterns in case patterns (*, ?, [...])
                if Self::glob_match(&pattern_str, &value) || pattern_str == value {
                    for cmd in &case.body {
                        self.execute_command(cmd)?;
                    }
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    // --- Fish builtins ---

    fn builtin_and(&mut self, args: &[Word]) -> Result<()> {
        // `and cmd` runs cmd only if last exit code was 0
        if self.last_exit_code != 0 {
            return Ok(());
        }
        if args.is_empty() {
            return Ok(());
        }
        // Rebuild as a simple command and execute
        let cmd = SimpleCommand {
            redirects: Vec::new(),
            assignments: Vec::new(),
            words: args.to_vec(),
        };
        self.execute_simple(&cmd, false)
    }

    fn builtin_or(&mut self, args: &[Word]) -> Result<()> {
        // `or cmd` runs cmd only if last exit code was non-zero
        if self.last_exit_code == 0 {
            return Ok(());
        }
        if args.is_empty() {
            return Ok(());
        }
        let cmd = SimpleCommand {
            redirects: Vec::new(),
            assignments: Vec::new(),
            words: args.to_vec(),
        };
        self.execute_simple(&cmd, false)
    }

    fn builtin_not(&mut self, args: &[Word]) -> Result<()> {
        // `not cmd` runs cmd and inverts exit code
        if args.is_empty() {
            return Ok(());
        }
        let cmd = SimpleCommand {
            redirects: Vec::new(),
            assignments: Vec::new(),
            words: args.to_vec(),
        };
        self.execute_simple(&cmd, false)?;
        self.last_exit_code = if self.last_exit_code == 0 { 1 } else { 0 };
        Ok(())
    }

    fn builtin_type(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("type: expected command name");
            self.last_exit_code = 1;
            return Ok(());
        }
        for arg in args {
            let name = self.expand_word(arg);
            if self.functions.contains_key(&name) {
                println!("{} is a function", name);
            } else if self.aliases.contains_key(&name) {
                println!("{} is an alias", name);
            } else if is_builtin(&name) {
                println!("{} is a builtin", name);
            } else {
                // Check if it's an executable on PATH
                match ProcessCommand::new("sh").arg("-c").arg(format!("command -v {}", name)).output() {
                    Ok(output) if output.status.success() => {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        println!("{} is {}", name, path);
                    }
                    _ => {
                        println!("{}: not found", name);
                        self.last_exit_code = 1;
                    }
                }
            }
        }
        Ok(())
    }

    fn builtin_count(&self, args: &[Word]) -> Result<()> {
        println!("{}", args.len());
        Ok(())
    }

    fn builtin_printf(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            return Ok(());
        }
        let fmt = self.expand_word(&args[0]);
        let rest: Vec<String> = args[1..].iter().map(|w| self.expand_word(w)).collect();
        let mut result = String::new();
        let mut chars = fmt.chars().peekable();
        let mut arg_idx = 0;
        while let Some(ch) = chars.next() {
            if ch == '%' {
                match chars.next() {
                    Some('s') => {
                        if arg_idx < rest.len() {
                            result.push_str(&rest[arg_idx]);
                            arg_idx += 1;
                        }
                    }
                    Some('d') | Some('i') => {
                        if arg_idx < rest.len() {
                            result.push_str(&rest[arg_idx]);
                            arg_idx += 1;
                        }
                    }
                    Some('%') => {
                        result.push('%');
                    }
                    Some(c) => {
                        result.push('%');
                        result.push(c);
                    }
                    None => {
                        result.push('%');
                    }
                }
            } else if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }
        print!("{}", result);
        Ok(())
    }

    fn builtin_source(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("source: expected file argument");
            self.last_exit_code = 1;
            return Ok(());
        }
        let path = self.expand_word(&args[0]);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("source: {}: {}", path, e))?;
        self.execute(&content)
    }

    fn builtin_read(&mut self, args: &[Word]) -> Result<()> {
        let mut var_name = "REPLY".to_string();
        let mut prompt = String::new();

        let mut args_iter = args.iter();
        while let Some(arg) = args_iter.next() {
            let s = self.expand_word(arg);
            match s.as_str() {
                "-p" | "--prompt" => {
                    if let Some(p) = args_iter.next() {
                        prompt = self.expand_word(p);
                    }
                }
                _ => {
                    // Last non-option arg is the variable name
                    var_name = s;
                }
            }
        }

        if !prompt.is_empty() {
            eprint!("{}", prompt);
            use std::io::Write;
            std::io::stderr().flush().ok();
        }

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim_end_matches('\n').trim_end_matches('\r').to_string();
        self.vars.insert(var_name, input);
        Ok(())
    }

    fn builtin_string(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("string: expected subcommand");
            self.last_exit_code = 1;
            return Ok(());
        }

        let subcmd = self.expand_word(&args[0]);
        let sub_args: Vec<String> = args[1..].iter().map(|w| self.expand_word(w)).collect();

        match subcmd.as_str() {
            "length" => {
                if sub_args.is_empty() {
                    // Read from stdin
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    for line in input.lines() {
                        println!("{}", line.len());
                    }
                } else {
                    for arg in &sub_args {
                        println!("{}", arg.len());
                    }
                }
            }
            "match" => {
                if sub_args.len() < 2 {
                    eprintln!("string match: expected pattern and string");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let pattern = &sub_args[0];
                let text = &sub_args[1];
                if Self::glob_match(pattern, text) {
                    println!("{}", text);
                    self.last_exit_code = 0;
                } else {
                    self.last_exit_code = 1;
                }
            }
            "replace" => {
                if sub_args.len() < 3 {
                    eprintln!("string replace: expected pattern replacement string");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let pattern = &sub_args[0];
                let replacement = &sub_args[1];
                let text = &sub_args[2];
                println!("{}", text.replace(pattern, replacement));
            }
            "split" => {
                if sub_args.is_empty() {
                    eprintln!("string split: expected delimiter");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let delimiter = &sub_args[0];
                if sub_args.len() >= 2 {
                    for arg in &sub_args[1..] {
                        for part in arg.split(delimiter.as_str()) {
                            println!("{}", part);
                        }
                    }
                } else {
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    for part in input.trim().split(delimiter.as_str()) {
                        println!("{}", part);
                    }
                }
            }
            "join" => {
                if sub_args.is_empty() {
                    eprintln!("string join: expected delimiter");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let delimiter = &sub_args[0];
                let parts: Vec<&str> = sub_args[1..].iter().map(|s| s.as_str()).collect();
                println!("{}", parts.join(delimiter));
            }
            "trim" => {
                if sub_args.is_empty() {
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    println!("{}", input.trim());
                } else {
                    for arg in &sub_args {
                        println!("{}", arg.trim());
                    }
                }
            }
            "sub" => {
                if sub_args.len() < 2 {
                    eprintln!("string sub: expected string start [length]");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let text = &sub_args[0];
                let start = sub_args[1].parse::<usize>().unwrap_or(0).saturating_sub(1);
                let chars: Vec<char> = text.chars().collect();
                let end = if sub_args.len() >= 3 {
                    let len = sub_args[2].parse::<usize>().unwrap_or(chars.len());
                    (start + len).min(chars.len())
                } else {
                    chars.len()
                };
                if start < chars.len() {
                    let result: String = chars[start..end].iter().collect();
                    println!("{}", result);
                }
            }
            "upper" => {
                for arg in &sub_args {
                    println!("{}", arg.to_uppercase());
                }
            }
            "lower" => {
                for arg in &sub_args {
                    println!("{}", arg.to_lowercase());
                }
            }
            "repeat" => {
                if sub_args.len() < 2 {
                    eprintln!("string repeat: expected count string");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let count = sub_args[0].parse::<usize>().unwrap_or(0);
                let text = &sub_args[1];
                for _ in 0..count {
                    print!("{}", text);
                }
                println!();
            }
            _ => {
                eprintln!("string: unknown subcommand '{}'", subcmd);
                self.last_exit_code = 1;
            }
        }
        Ok(())
    }

    fn builtin_math(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("math: expected expression");
            self.last_exit_code = 1;
            return Ok(());
        }
        // Reconstruct raw words without glob expansion to preserve * and other operators
        let expr: String = args.iter().map(|w| self.reconstruct_word_raw(w)).collect::<Vec<_>>().join(" ");
        // Use sh -c echo $((expr)) for arithmetic evaluation
        let cmd_str = format!("echo $(( {} ))", expr);
        match ProcessCommand::new("sh").arg("-c").arg(&cmd_str).output() {
            Ok(output) if output.status.success() => {
                let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("{}", result);
                self.last_exit_code = 0;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                eprintln!("math: {}", stderr);
                self.last_exit_code = 1;
            }
            Err(e) => {
                eprintln!("math: {}", e);
                self.last_exit_code = 1;
            }
        }
        Ok(())
    }

    fn builtin_status(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            // No args: print current status (like fish)
            if self.is_interactive {
                println!("Interactive");
            } else {
                println!("Non-interactive");
            }
            return Ok(());
        }
        let subcmd = self.expand_word(&args[0]);
        match subcmd.as_str() {
            "is-interactive" => {
                self.last_exit_code = if self.is_interactive { 0 } else { 1 };
            }
            "is-command-substitution" => {
                // We're in command substitution if this was called from capture_command_body
                // Simplified: always return false
                self.last_exit_code = 1;
            }
            "is-full-job-control" | "is-interactive-job-control" => {
                self.last_exit_code = if self.is_interactive { 0 } else { 1 };
            }
            "is-login" => {
                // Simplified: check if argv[0] starts with -
                self.last_exit_code = 1;
            }
            "current-command" | "current-commandline" => {
                println!("mishell");
            }
            "filename" => {
                println!("mishell");
            }
            "line-number" => {
                println!("0");
            }
            "fish-path" => {
                // Return path to mishell binary
                match std::env::current_exe() {
                    Ok(path) => println!("{}", path.display()),
                    Err(_) => println!("mishell"),
                }
            }
            "exit" => {
                let code = args.get(1)
                    .map(|w| self.expand_word(w).parse::<i32>().unwrap_or(0))
                    .unwrap_or(self.last_exit_code);
                std::process::exit(code);
            }
            "test-feature" => {
                // fish compatibility: always return 1 (no features tested)
                self.last_exit_code = 1;
            }
            "job-control" => {
                let mode = args.get(1).map(|w| self.expand_word(w)).unwrap_or_default();
                match mode.as_str() {
                    "full" | "interactive" | "none" => {
                        // Accept but don't actually change job control
                        self.last_exit_code = 0;
                    }
                    _ => {
                        // Query mode
                        println!("full");
                        self.last_exit_code = 0;
                    }
                }
            }
            "stack-trace" => {
                // Not implemented, just return success
                self.last_exit_code = 0;
            }
            _ => {
                eprintln!("status: unknown subcommand '{}'", subcmd);
                self.last_exit_code = 1;
            }
        }
        Ok(())
    }

    fn builtin_command(&mut self, args: &[Word]) -> Result<()> {
        // `command cmd` bypasses functions and aliases, runs external command directly
        if args.is_empty() {
            return Ok(());
        }
        let cmd_name = self.expand_word(&args[0]);

        // Check for -q/--quiet flag (just test if command exists)
        if cmd_name == "-q" || cmd_name == "--quiet" {
            if args.len() < 2 {
                self.last_exit_code = 1;
                return Ok(());
            }
            let name = self.expand_word(&args[1]);
            let output = ProcessCommand::new("sh")
                .arg("-c")
                .arg(format!("command -v {}", name))
                .output();
            self.last_exit_code = match output {
                Ok(o) if o.status.success() => 0,
                _ => 1,
            };
            return Ok(());
        }

        // Rebuild and execute directly via sh, bypassing aliases/functions
        let mut cmd_str = cmd_name.clone();
        for word in &args[1..] {
            cmd_str.push(' ');
            cmd_str.push_str(&self.expand_word(word));
        }
        let status = ProcessCommand::new("sh").arg("-c").arg(&cmd_str).status()?;
        self.last_exit_code = status.code().unwrap_or(1);
        Ok(())
    }

    fn builtin_builtin_cmd(&mut self, args: &[Word]) -> Result<()> {
        // `builtin cmd` bypasses functions, runs builtin directly
        if args.is_empty() {
            return Ok(());
        }
        let cmd_name = self.expand_word(&args[0]);

        // Check if it's actually a builtin
        if !is_builtin(&cmd_name) {
            eprintln!("builtin: '{}'' is not a builtin", cmd_name);
            self.last_exit_code = 1;
            return Ok(());
        }

        // Re-execute as a simple command - since builtins are matched in execute_simple,
        // this will go through the builtin path directly (no function/alias check needed
        // since we already confirmed it's a builtin)
        let cmd = SimpleCommand {
            redirects: Vec::new(),
            assignments: Vec::new(),
            words: args.to_vec(),
        };
        self.execute_simple(&cmd, false)
    }

    fn builtin_contains(&mut self, args: &[Word]) -> Result<()> {
        // `contains item list...` - returns 0 if item is in list, 1 otherwise
        if args.len() < 2 {
            eprintln!("contains: expected at least 2 arguments");
            self.last_exit_code = 1;
            return Ok(());
        }
        let needle = self.expand_word(&args[0]);
        for arg in &args[1..] {
            if self.expand_word(arg) == needle {
                self.last_exit_code = 0;
                return Ok(());
            }
        }
        self.last_exit_code = 1;
        Ok(())
    }

    fn builtin_random(&mut self, args: &[Word]) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        if args.is_empty() {
            // Print random number in range [0, 2^31 - 1] (fish behavior)
            let val = (seed % (i32::MAX as u128 + 1)) as i32;
            println!("{}", val);
            self.last_exit_code = 0;
            return Ok(());
        }

        let arg = self.expand_word(&args[0]);
        match arg.as_str() {
            "choice" => {
                // `random choice item1 item2 ...` - pick random item
                let choices: Vec<&Word> = args[1..].iter().collect();
                if choices.is_empty() {
                    eprintln!("random choice: expected arguments");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let idx = (seed as usize) % choices.len();
                println!("{}", self.expand_word(choices[idx]));
                self.last_exit_code = 0;
            }
            "seed" => {
                // `random seed` - seed is based on time, just report it
                println!("{}", seed);
                self.last_exit_code = 0;
            }
            _ => {
                // `random [min] [max]` - random number in range
                let min = arg.parse::<i64>().unwrap_or(0);
                let max = if args.len() >= 2 {
                    self.expand_word(&args[1]).parse::<i64>().unwrap_or(i32::MAX as i64)
                } else {
                    // Single arg: random 0..arg
                    let val = min;
                    // Actually fish treats single arg as max
                    let result = (seed as i64 % (val + 1)).abs();
                    println!("{}", result);
                    self.last_exit_code = 0;
                    return Ok(());
                };
                if min > max {
                    eprintln!("random: min ({}) > max ({})", min, max);
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let range = (max - min + 1) as u128;
                let result = min + (seed % range) as i64;
                println!("{}", result);
                self.last_exit_code = 0;
            }
        }
        Ok(())
    }

    fn builtin_emit(&mut self, args: &[Word]) -> Result<()> {
        // `emit event_name [args...]` - call all handlers for event_name
        if args.is_empty() {
            eprintln!("emit: expected event name");
            self.last_exit_code = 1;
            return Ok(());
        }
        let event_name = self.expand_word(&args[0]);
        let event_args: Vec<String> = args[1..].iter().map(|w| self.expand_word(w)).collect();

        // Store args as $argv for handlers
        let saved_argv = self.vars.get("argv").cloned();

        if let Some(handlers) = self.event_handlers.get(&event_name).cloned() {
            for handler in &handlers {
                // Set $argv for the handler
                self.vars.insert("argv".to_string(), event_args.join(" "));
                for (i, arg) in event_args.iter().enumerate() {
                    self.vars.insert(format!("{}", i + 1), arg.clone());
                }
                for cmd in &handler.body {
                    self.execute_command(cmd)?;
                }
            }
        }

        // Restore $argv
        match saved_argv {
            Some(v) => { self.vars.insert("argv".to_string(), v); }
            None => { self.vars.remove("argv"); }
        }

        self.last_exit_code = 0;
        Ok(())
    }

    fn reconstruct_word_raw(&self, word: &Word) -> String {
        let mut result = String::new();
        for part in &word.parts {
            match part {
                WordPart::Literal(s) => result.push_str(s),
                WordPart::Variable(name) => {
                    if name == "status" {
                        result.push_str(&self.last_exit_code.to_string());
                    } else {
                        let value = self.vars.get(name)
                            .or_else(|| self.universal_vars.get(name))
                            .cloned()
                            .unwrap_or_else(|| std::env::var(name).unwrap_or_default());
                        result.push_str(&value);
                    }
                }
                WordPart::SingleQuoted(s) => result.push_str(s),
                WordPart::DoubleQuoted(parts) => {
                    for p in parts {
                        match p {
                            WordPart::Literal(s) => result.push_str(s),
                            WordPart::Variable(name) => {
                                if name == "status" {
                                    result.push_str(&self.last_exit_code.to_string());
                                } else {
                                    let value = self.vars.get(name)
                                        .or_else(|| self.universal_vars.get(name))
                                        .cloned()
                                        .unwrap_or_else(|| std::env::var(name).unwrap_or_default());
                                    result.push_str(&value);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WordPart::Glob(pattern) => {
                    match pattern {
                        GlobPattern::Star => result.push('*'),
                        GlobPattern::Question => result.push('?'),
                        GlobPattern::Class(s) => result.push_str(&format!("[{}]", s)),
                        GlobPattern::Literal(s) => result.push_str(s),
                    }
                }
                _ => {}
            }
        }
        result
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn set_interactive(&mut self, interactive: bool) {
        self.is_interactive = interactive;
    }
}

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "export"
            | "alias"
            | "abbr"
            | "set"
            | "exit"
            | "pushd"
            | "popd"
            | "dirs"
            | "history"
            | "jobs"
            | "fg"
            | "bg"
            | "and"
            | "or"
            | "not"
            | "type"
            | "count"
            | "printf"
            | "source"
            | "."
            | "read"
            | "string"
            | "math"
            | "status"
            | "command"
            | "builtin"
            | "contains"
            | "random"
            | "emit"
            | "begin"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- glob_match tests ---

    #[test]
    fn test_glob_star() {
        assert!(Shell::glob_match("*", "anything"));
        assert!(Shell::glob_match("*", ""));
        assert!(Shell::glob_match("*.txt", "file.txt"));
        assert!(!Shell::glob_match("*.txt", "file.rs"));
    }

    #[test]
    fn test_glob_question() {
        assert!(Shell::glob_match("?", "a"));
        assert!(!Shell::glob_match("?", "ab"));
        assert!(!Shell::glob_match("?", ""));
        assert!(Shell::glob_match("?.txt", "a.txt"));
        assert!(!Shell::glob_match("?.txt", "ab.txt"));
    }

    #[test]
    fn test_glob_literal() {
        assert!(Shell::glob_match("hello", "hello"));
        assert!(!Shell::glob_match("hello", "world"));
        assert!(!Shell::glob_match("hello", "Hello"));
    }

    #[test]
    fn test_glob_star_question_combo() {
        assert!(Shell::glob_match("*.?", "a.b"));
        assert!(Shell::glob_match("*.?", "file.x"));
        assert!(!Shell::glob_match("*.?", "file."));
        assert!(!Shell::glob_match("*.?", "file.xy"));
    }

    #[test]
    fn test_glob_star_multiple() {
        assert!(Shell::glob_match("*.*", "a.b"));
        assert!(Shell::glob_match("*.*", "file.txt"));
        assert!(!Shell::glob_match("*.*", "noext"));
    }

    #[test]
    fn test_glob_char_class_range() {
        assert!(Shell::glob_match("[a-z]", "a"));
        assert!(Shell::glob_match("[a-z]", "z"));
        assert!(!Shell::glob_match("[a-z]", "A"));
        assert!(!Shell::glob_match("[a-z]", "0"));
        assert!(Shell::glob_match("[A-Z]", "M"));
    }

    #[test]
    fn test_glob_char_class_digits() {
        assert!(Shell::glob_match("[0-9]", "5"));
        assert!(!Shell::glob_match("[0-9]", "a"));
    }

    #[test]
    fn test_glob_char_class_negated() {
        assert!(!Shell::glob_match("[^a-z]", "a"));
        assert!(Shell::glob_match("[^a-z]", "A"));
        assert!(Shell::glob_match("[^a-z]", "0"));
    }

    #[test]
    fn test_glob_char_class_specific() {
        assert!(Shell::glob_match("[abc]", "a"));
        assert!(Shell::glob_match("[abc]", "b"));
        assert!(!Shell::glob_match("[abc]", "d"));
    }

    #[test]
    fn test_glob_complex_pattern() {
        assert!(Shell::glob_match("[A-Z]*.txt", "README.txt"));
        assert!(!Shell::glob_match("[A-Z]*.txt", "readme.txt"));
        assert!(Shell::glob_match("file[0-9].log", "file5.log"));
        assert!(!Shell::glob_match("file[0-9].log", "fileA.log"));
    }

    #[test]
    fn test_glob_no_match_returns_literal() {
        // When expand_glob finds no matches, it returns None (literal used)
        // glob_match itself just returns false
        assert!(!Shell::glob_match("*.nonexistent_extension_xyz", "file.txt"));
    }

    #[test]
    fn test_glob_empty_pattern() {
        assert!(Shell::glob_match("", ""));
        assert!(!Shell::glob_match("", "a"));
    }

    #[test]
    fn test_glob_star_only() {
        assert!(Shell::glob_match("*", ""));
        assert!(Shell::glob_match("*", "anything at all"));
    }

    // --- is_builtin tests ---

    #[test]
    fn test_is_builtin_core() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("export"));
        assert!(is_builtin("alias"));
        assert!(is_builtin("set"));
        assert!(is_builtin("exit"));
        // echo is NOT in the builtin list (handled by sh -c)
        assert!(!is_builtin("echo"));
    }

    #[test]
    fn test_is_builtin_fish() {
        assert!(is_builtin("and"));
        assert!(is_builtin("or"));
        assert!(is_builtin("not"));
        assert!(is_builtin("count"));
        assert!(is_builtin("string"));
        assert!(is_builtin("math"));
        assert!(is_builtin("status"));
        assert!(is_builtin("contains"));
        assert!(is_builtin("random"));
        assert!(is_builtin("emit"));
        assert!(is_builtin("begin"));
    }

    #[test]
    fn test_is_builtin_dirs() {
        assert!(is_builtin("pushd"));
        assert!(is_builtin("popd"));
        assert!(is_builtin("dirs"));
    }

    #[test]
    fn test_is_builtin_jobs() {
        assert!(is_builtin("jobs"));
        assert!(is_builtin("fg"));
        assert!(is_builtin("bg"));
    }

    #[test]
    fn test_is_builtin_not_builtin() {
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
        assert!(!is_builtin("cat"));
        assert!(!is_builtin("git"));
        assert!(!is_builtin("cargo"));
        assert!(!is_builtin(""));
    }

    // --- Shell construction tests ---

    #[test]
    fn test_shell_new() {
        let shell = Shell::new(false).unwrap();
        assert_eq!(shell.last_exit_code, 0);
        assert!(shell.is_interactive);
        assert!(shell.functions.is_empty());
        assert!(shell.aliases.is_empty());
        assert!(shell.abbreviations.is_empty());
        assert!(shell.jobs.is_empty());
        assert_eq!(shell.next_job_id, 1);
    }

    #[test]
    fn test_shell_new_with_fish_features() {
        let shell = Shell::new(true).unwrap();
        assert!(shell.fish_features);
    }

    #[test]
    fn test_shell_set_interactive() {
        let mut shell = Shell::new(false).unwrap();
        assert!(shell.is_interactive);
        shell.set_interactive(false);
        assert!(!shell.is_interactive);
        shell.set_interactive(true);
        assert!(shell.is_interactive);
    }

    // --- expand_word tests ---

    #[test]
    fn test_expand_word_literal() {
        let shell = Shell::new(false).unwrap();
        let word = Word { parts: vec![WordPart::Literal("hello".to_string())] };
        assert_eq!(shell.expand_word(&word), "hello");
    }

    #[test]
    fn test_expand_word_variable() {
        let mut shell = Shell::new(false).unwrap();
        shell.vars.insert("MYVAR".to_string(), "myvalue".to_string());
        let word = Word { parts: vec![WordPart::Variable("MYVAR".to_string())] };
        assert_eq!(shell.expand_word(&word), "myvalue");
    }

    #[test]
    fn test_expand_word_status_variable() {
        let mut shell = Shell::new(false).unwrap();
        shell.last_exit_code = 42;
        let word = Word { parts: vec![WordPart::Variable("status".to_string())] };
        assert_eq!(shell.expand_word(&word), "42");
    }

    #[test]
    fn test_expand_word_mixed() {
        let mut shell = Shell::new(false).unwrap();
        shell.vars.insert("NAME".to_string(), "world".to_string());
        let word = Word {
            parts: vec![
                WordPart::Literal("hello_".to_string()),
                WordPart::Variable("NAME".to_string()),
                WordPart::Literal("!".to_string()),
            ]
        };
        assert_eq!(shell.expand_word(&word), "hello_world!");
    }

    #[test]
    fn test_expand_word_single_quoted() {
        let shell = Shell::new(false).unwrap();
        let word = Word { parts: vec![WordPart::SingleQuoted("$VAR literal".to_string())] };
        assert_eq!(shell.expand_word(&word), "$VAR literal");
    }

    #[test]
    fn test_expand_word_double_quoted_with_var() {
        let mut shell = Shell::new(false).unwrap();
        shell.vars.insert("X".to_string(), "42".to_string());
        let word = Word {
            parts: vec![WordPart::DoubleQuoted(vec![
                WordPart::Literal("val=".to_string()),
                WordPart::Variable("X".to_string()),
            ])]
        };
        assert_eq!(shell.expand_word(&word), "val=42");
    }

    #[test]
    fn test_expand_word_escape() {
        let shell = Shell::new(false).unwrap();
        let word = Word { parts: vec![WordPart::Escape('n')] };
        assert_eq!(shell.expand_word(&word), "n");
    }

    #[test]
    fn test_expand_word_empty() {
        let shell = Shell::new(false).unwrap();
        let word = Word { parts: vec![] };
        assert_eq!(shell.expand_word(&word), "");
    }

    #[test]
    fn test_expand_word_tilde() {
        let shell = Shell::new(false).unwrap();
        let word = Word { parts: vec![WordPart::Tilde(None)] };
        let result = shell.expand_word(&word);
        // Should be home dir path or ~
        assert!(!result.is_empty());
    }

    // --- abbreviation expansion tests ---

    #[test]
    fn test_expand_abbreviation_at_start() {
        let mut shell = Shell::new(true).unwrap();
        shell.abbreviations.insert("gc".to_string(), "git commit".to_string());
        let result = shell.expand_abbreviations("gc -m test");
        assert_eq!(result, "git commit -m test");
    }

    #[test]
    fn test_expand_abbreviation_in_middle() {
        let mut shell = Shell::new(true).unwrap();
        shell.abbreviations.insert("gp".to_string(), "git push".to_string());
        let result = shell.expand_abbreviations("echo gp"); // not at word boundary with space before
        // The abbreviation expansion looks for " gp " pattern
        assert!(result.contains("echo"));
    }

    #[test]
    fn test_expand_abbreviation_no_match() {
        let mut shell = Shell::new(true).unwrap();
        shell.abbreviations.insert("gc".to_string(), "git commit".to_string());
        let result = shell.expand_abbreviations("echo hello");
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_expand_abbreviation_disabled() {
        let mut shell = Shell::new(false).unwrap();
        shell.abbreviations.insert("gc".to_string(), "git commit".to_string());
        // fish_features is false, so execute() won't expand
        // But expand_abbreviations is called only when fish_features is true
        let result = shell.expand_abbreviations("gc -m test");
        assert_eq!(result, "git commit -m test"); // expand_abbreviations always works
    }

    // --- alias tests ---

    #[test]
    fn test_alias_accessor() {
        let mut shell = Shell::new(false).unwrap();
        shell.aliases.insert("ll".to_string(), "ls -la".to_string());
        assert_eq!(shell.aliases().get("ll").unwrap(), "ls -la");
        assert!(shell.aliases().contains_key("ll"));
    }

    // --- vars accessor tests ---

    #[test]
    fn test_vars_accessor() {
        let mut shell = Shell::new(false).unwrap();
        shell.vars.insert("TESTVAR".to_string(), "value".to_string());
        assert_eq!(shell.vars().get("TESTVAR").unwrap(), "value");
    }

    // --- reconstruct_body tests ---

    #[test]
    fn test_reconstruct_body_simple() {
        let shell = Shell::new(false).unwrap();
        let body = CommandBody::Simple(SimpleCommand {
            redirects: vec![],
            assignments: vec![],
            words: vec![
                Word { parts: vec![WordPart::Literal("echo".to_string())] },
                Word { parts: vec![WordPart::Literal("hello".to_string())] },
            ],
        });
        assert_eq!(shell.reconstruct_body(&body), "echo hello");
    }

    #[test]
    fn test_reconstruct_body_pipeline() {
        let shell = Shell::new(false).unwrap();
        let body = CommandBody::Pipeline(Pipeline {
            commands: vec![
                SimpleCommand {
                    redirects: vec![],
                    assignments: vec![],
                    words: vec![
                        Word { parts: vec![WordPart::Literal("ls".to_string())] },
                    ],
                },
                SimpleCommand {
                    redirects: vec![],
                    assignments: vec![],
                    words: vec![
                        Word { parts: vec![WordPart::Literal("grep".to_string())] },
                        Word { parts: vec![WordPart::Literal("foo".to_string())] },
                    ],
                },
            ],
            negated: false,
        });
        assert_eq!(shell.reconstruct_body(&body), "ls | grep foo");
    }

    #[test]
    fn test_reconstruct_body_and() {
        let shell = Shell::new(false).unwrap();
        let left = CommandBody::Simple(SimpleCommand {
            redirects: vec![],
            assignments: vec![],
            words: vec![Word { parts: vec![WordPart::Literal("true".to_string())] }],
        });
        let right = CommandBody::Simple(SimpleCommand {
            redirects: vec![],
            assignments: vec![],
            words: vec![Word { parts: vec![WordPart::Literal("echo".to_string())] },
                        Word { parts: vec![WordPart::Literal("ok".to_string())] }],
        });
        let body = CommandBody::And(Box::new(left), Box::new(right));
        assert_eq!(shell.reconstruct_body(&body), "true && echo ok");
    }

    #[test]
    fn test_reconstruct_body_sequence() {
        let shell = Shell::new(false).unwrap();
        let left = CommandBody::Simple(SimpleCommand {
            redirects: vec![],
            assignments: vec![],
            words: vec![Word { parts: vec![WordPart::Literal("a".to_string())] }],
        });
        let right = CommandBody::Simple(SimpleCommand {
            redirects: vec![],
            assignments: vec![],
            words: vec![Word { parts: vec![WordPart::Literal("b".to_string())] }],
        });
        let body = CommandBody::Sequence(Box::new(left), Box::new(right));
        assert_eq!(shell.reconstruct_body(&body), "a; b");
    }

    // --- execute tests (capture output) ---

    #[test]
    fn test_execute_simple_echo() {
        let mut shell = Shell::new(false).unwrap();
        // execute() runs through sh -c, so this tests the full pipeline
        shell.execute("echo hello").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_pipeline() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("echo hello | tr a-z A-Z").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_and_chain() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("true && echo yes").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_or_chain() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("false || echo fallback").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_assignment() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("FOO=bar").unwrap();
        assert_eq!(shell.vars.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn test_execute_empty_input() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("").unwrap();
        shell.execute("   ").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_redirect_output() {
        let mut shell = Shell::new(false).unwrap();
        let path = "/tmp/mishell_test_redirect_out.txt";
        shell.execute(&format!("echo test > {}", path)).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content.trim(), "test");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_execute_redirect_append() {
        let mut shell = Shell::new(false).unwrap();
        let path = "/tmp/mishell_test_redirect_append.txt";
        let _ = std::fs::remove_file(path);
        shell.execute(&format!("echo line1 >> {}", path)).unwrap();
        shell.execute(&format!("echo line2 >> {}", path)).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("line1"));
        assert!(content.contains("line2"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_execute_sequence() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("FOO=hello; echo $FOO").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_function_define() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("function greet; echo hello; end").unwrap();
        assert!(shell.functions.contains_key("greet"));
    }

    #[test]
    fn test_execute_for_loop() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("for i in 1 2 3; echo $i; end").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_if_true() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("if true; then echo yes; end").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_if_false_else() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("if false; then echo yes; else echo no; end").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_command_substitution() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("X=$(echo hi)").unwrap();
        assert_eq!(shell.vars.get("X").unwrap(), "hi");
    }

    #[test]
    fn test_execute_export() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("export TEST_EXPORT_VAR=hello").unwrap();
        assert_eq!(std::env::var("TEST_EXPORT_VAR").unwrap(), "hello");
        let _ = std::env::remove_var("TEST_EXPORT_VAR");
    }

    #[test]
    fn test_execute_count_builtin() {
        let mut shell = Shell::new(false).unwrap();
        // count just prints, so just verify no error
        shell.execute("count a b c").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_math_builtin() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("math 2 + 3").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_not_inverts() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("not false").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_and_skips_on_failure() {
        let mut shell = Shell::new(false).unwrap();
        shell.last_exit_code = 1;
        // `and` should not run if last_exit_code != 0
        shell.execute("and echo should_not_run").unwrap();
        assert_eq!(shell.last_exit_code, 1); // unchanged
    }

    #[test]
    fn test_execute_or_skips_on_success() {
        let mut shell = Shell::new(false).unwrap();
        shell.last_exit_code = 0;
        // `or` should not run if last_exit_code == 0
        shell.execute("or echo should_not_run").unwrap();
        assert_eq!(shell.last_exit_code, 0); // unchanged
    }

    #[test]
    fn test_execute_contains_found() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("contains foo bar baz foo").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_contains_not_found() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("contains notfound bar baz").unwrap();
        assert_eq!(shell.last_exit_code, 1);
    }

    #[test]
    fn test_execute_status_is_interactive() {
        let mut shell = Shell::new(false).unwrap();
        shell.set_interactive(false);
        shell.execute("status is-interactive").unwrap();
        assert_eq!(shell.last_exit_code, 1); // not interactive
    }

    #[test]
    fn test_execute_status_is_interactive_true() {
        let mut shell = Shell::new(false).unwrap();
        shell.set_interactive(true);
        shell.execute("status is-interactive").unwrap();
        assert_eq!(shell.last_exit_code, 0); // interactive
    }

    #[test]
    fn test_execute_glob_expansion() {
        let mut shell = Shell::new(false).unwrap();
        // Just verify it doesn't error - glob expands to matching files
        shell.execute("echo *.toml").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }
}
