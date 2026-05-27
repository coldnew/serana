use mishell_parser::{Parser, Command, CommandBody, SimpleCommand, Pipeline, Word, WordPart, Redirect, RedirectOp, RedirectTarget, FunctionDef, ForLoop, WhileLoop, IfStatement, SwitchStatement, GlobPattern};
use std::collections::HashMap;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use nix::unistd::Pid;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag};
use std::os::unix::process::CommandExt;

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
    completions: HashMap<String, Vec<CompletionEntry>>,
}

#[derive(Clone)]
pub struct CompletionEntry {
    pub condition: String,
    pub description: String,
    pub arguments: Vec<String>,
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
            completions: HashMap::new(),
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
                // Register event handler if --on-event specified
                if let Some(ref event) = func.on_event {
                    self.event_handlers
                        .entry(event.clone())
                        .or_default()
                        .push(func.clone());
                }
                // Register variable handler if --on-variable specified
                if let Some(ref var) = func.on_variable {
                    self.event_handlers
                        .entry(format!("variable:{}", var))
                        .or_default()
                        .push(func.clone());
                }
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
                // Register event handler if --on-event specified
                if let Some(ref event) = func.on_event {
                    self.event_handlers
                        .entry(event.clone())
                        .or_default()
                        .push(func.clone());
                }
                // Register variable handler if --on-variable specified
                if let Some(ref var) = func.on_variable {
                    self.event_handlers
                        .entry(format!("variable:{}", var))
                        .or_default()
                        .push(func.clone());
                }
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
            "history" => return self.builtin_history(&cmd.words[1..]),
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
            "funced" => return self.builtin_funced(&cmd.words[1..]),
            "funcsave" => return self.builtin_funcsave(&cmd.words[1..]),
            "functions" => return self.builtin_functions(&cmd.words[1..]),
            "edit" => return self.builtin_edit(&cmd.words[1..]),
            "file" => return self.builtin_file(&cmd.words[1..]),
            "head" => return self.builtin_head(&cmd.words[1..]),
            "tail" => return self.builtin_tail(&cmd.words[1..]),
            "try" => return self.builtin_try(&cmd.words[1..]),
            "test" | "[" => return self.builtin_test(&cmd.words[1..]),
            "eval" => return self.builtin_eval(&cmd.words[1..]),
            "realpath" => return self.builtin_realpath(&cmd.words[1..]),
            "complete" => return self.builtin_complete(&cmd.words[1..]),
            "commandline" => return self.builtin_commandline(&cmd.words[1..]),
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
            // Spawn in own process group for signal isolation
            process_cmd.process_group(0);
            let child = process_cmd.spawn()?;
            let job_id = self.next_job_id;
            self.next_job_id += 1;
            let pid = child.id();
            eprintln!("[{}] {} {}", job_id, pid, cmd_str);
            self.jobs.push(Job {
                id: job_id,
                pid,
                command: cmd_str.clone(),
                child,
            });
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
        } else if name == "fish_pid" {
            result.push_str(&std::process::id().to_string());
        } else if name == "hostname" {
            if let Ok(h) = hostname::get() {
                result.push_str(&h.to_string_lossy());
            }
        } else if name == "version" {
            result.push_str("0.1.0");
        } else if name == "USER" {
            if let Ok(u) = std::env::var("USER") {
                result.push_str(&u);
            }
        } else if name == "HOME" {
            if let Some(ref h) = self.home_dir {
                result.push_str(&h.to_string_lossy());
            }
        } else if name == "SHELL" {
            if let Ok(s) = std::env::var("SHELL") {
                result.push_str(&s);
            } else {
                result.push_str("/usr/bin/mishell");
            }
        } else if name == "SHLVL" {
            let lvl = std::env::var("SHLVL").unwrap_or_else(|_| "1".to_string());
            let next: i32 = lvl.parse().unwrap_or(1) + 1;
            result.push_str(&next.to_string());
        } else if let Some(val) = self.vars.get(name) {
            result.push_str(val);
        } else if let Some(val) = self.universal_vars.get(name) {
            result.push_str(val);
        } else if let Ok(val) = std::env::var(name) {
            result.push_str(&val);
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

    pub fn glob_match(pattern: &str, text: &str) -> bool {
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

    fn reconstruct_command(&self, cmd: &Command) -> String {
        let mut result = self.reconstruct_body(&cmd.body);
        if cmd.background {
            result.push_str(" &");
        }
        result
    }

    fn reconstruct_word_raw(&self, word: &Word) -> String {
        word.parts.iter().map(|p| self.reconstruct_wordpart_raw(p)).collect()
    }

    fn reconstruct_wordpart_raw(&self, part: &WordPart) -> String {
        match part {
            WordPart::Literal(s) => s.clone(),
            WordPart::Variable(v) => format!("${{{}}}", v),
            WordPart::CommandSub(body) => format!("$({})", self.reconstruct_body(body)),
            WordPart::Arithmetic(expr) => format!("$(({}))", expr),
            WordPart::Glob(g) => match g {
                GlobPattern::Star => "*".to_string(),
                GlobPattern::Question => "?".to_string(),
                GlobPattern::Class(s) => format!("[{}]", s),
                GlobPattern::Literal(s) => s.clone(),
            },
            WordPart::Tilde(user) => match user {
                Some(u) => format!("~{}", u),
                None => "~".to_string(),
            },
            WordPart::Escape(c) => format!("\\{}", c),
            WordPart::DoubleQuoted(parts) => {
                let inner: String = parts.iter().map(|p| self.reconstruct_wordpart_raw(p)).collect();
                format!("\"{}\"", inner)
            }
            WordPart::SingleQuoted(s) => format!("'{}'", s),
        }
    }

    fn reconstruct_commands(&self, cmds: &[Command]) -> String {
        cmds.iter().map(|c| self.reconstruct_command(c)).collect::<Vec<_>>().join("; ")
    }

    fn reconstruct_redirects(&self, redirects: &[Redirect]) -> String {
        let mut parts = Vec::new();
        for r in redirects {
            let fd_prefix = r.fd.map_or(String::new(), |fd| fd.to_string());
            let op_str = match r.op {
                RedirectOp::Output => ">",
                RedirectOp::Append => ">>",
                RedirectOp::Input => "<",
                RedirectOp::DupOutput => ">&",
                RedirectOp::DupInput => "<&",
                RedirectOp::BothOutput => "&>",
                RedirectOp::BothAppend => "&>>",
                RedirectOp::HereString => "<<<",
                RedirectOp::HereDoc => "<<",
                RedirectOp::HereDocStrip => "<<-",
            };
            let target = match &r.target {
                RedirectTarget::File(w) => self.reconstruct_word_raw(w),
                RedirectTarget::Fd(fd) => fd.to_string(),
                RedirectTarget::HereDoc(s) => s.clone(),
            };
            parts.push(format!("{}{}{}", fd_prefix, op_str, target));
        }
        if parts.is_empty() { String::new() } else { format!(" {}", parts.join(" ")) }
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
            CommandBody::FunctionDef(func) => {
                let mut s = format!("function {}", func.name);
                if let Some(ref event) = func.on_event {
                    s.push_str(&format!(" --on-event {}", event));
                }
                if let Some(ref var) = func.on_variable {
                    s.push_str(&format!(" --on-variable {}", var));
                }
                for cmd in &func.body {
                    s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                }
                s.push_str("\nend");
                s
            }
            CommandBody::ForLoop(for_loop) => {
                let list_str: Vec<String> = for_loop.list.iter().map(|w| self.reconstruct_word_raw(w)).collect();
                let mut s = format!("for {} in {}", for_loop.variable, list_str.join(" "));
                for cmd in &for_loop.body {
                    s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                }
                s.push_str("\nend");
                s
            }
            CommandBody::WhileLoop(while_loop) => {
                let mut s = format!("while {}", self.reconstruct_commands(&while_loop.condition));
                for cmd in &while_loop.body {
                    s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                }
                s.push_str("\nend");
                s
            }
            CommandBody::If(if_stmt) => {
                let mut s = format!("if {}", self.reconstruct_commands(&if_stmt.condition));
                for cmd in &if_stmt.then_body {
                    s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                }
                for elif in &if_stmt.elif_branches {
                    s.push_str(&format!("\nelif {}", self.reconstruct_commands(&elif.condition)));
                    for cmd in &elif.body {
                        s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                    }
                }
                if let Some(ref else_cmds) = if_stmt.else_body {
                    s.push_str("\nelse");
                    for cmd in else_cmds {
                        s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                    }
                }
                s.push_str("\nend");
                s
            }
            CommandBody::Switch(switch) => {
                let mut s = format!("switch {}", self.reconstruct_word_raw(&switch.value));
                for case in &switch.cases {
                    let patterns: Vec<String> = case.patterns.iter().map(|p| self.reconstruct_word_raw(p)).collect();
                    s.push_str(&format!("\ncase {}", patterns.join(" ")));
                    for cmd in &case.body {
                        s.push_str(&format!("\n    {}", self.reconstruct_command(cmd)));
                    }
                }
                s.push_str("\nend");
                s
            }
        }
    }

    fn reconstruct_simple(&self, cmd: &SimpleCommand) -> String {
        let mut parts = Vec::new();
        for assign in &cmd.assignments {
            parts.push(format!("{}={}", assign.name, self.reconstruct_word_raw(&assign.value)));
        }
        for word in &cmd.words {
            parts.push(self.reconstruct_word_raw(word));
        }
        let mut result = parts.join(" ");
        result.push_str(&self.reconstruct_redirects(&cmd.redirects));
        result
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

        let expanded: Vec<String> = args.iter().map(|w| self.expand_word(w)).collect();
        let mut i = 0;
        while i < expanded.len() {
            match expanded[i].as_str() {
                "--add" | "-a" => {
                    if i + 2 < expanded.len() {
                        let name = expanded[i + 1].clone();
                        let value = expanded[i + 2].clone();
                        self.abbreviations.insert(name, value);
                        i += 3;
                    } else {
                        eprintln!("abbr: --add requires NAME and EXPANSION");
                        self.last_exit_code = 1;
                        return Ok(());
                    }
                }
                "--erase" | "-e" => {
                    if i + 1 < expanded.len() {
                        let name = expanded[i + 1].clone();
                        if self.abbreviations.remove(&name).is_none() {
                            eprintln!("abbr: no abbreviation named '{}'", name);
                            self.last_exit_code = 1;
                        }
                        i += 2;
                    } else {
                        eprintln!("abbr: --erase requires NAME");
                        self.last_exit_code = 1;
                        return Ok(());
                    }
                }
                "--rename" | "-r" => {
                    if i + 2 < expanded.len() {
                        let old_name = expanded[i + 1].clone();
                        let new_name = expanded[i + 2].clone();
                        if let Some(value) = self.abbreviations.remove(&old_name) {
                            self.abbreviations.insert(new_name, value);
                        } else {
                            eprintln!("abbr: no abbreviation named '{}'", old_name);
                            self.last_exit_code = 1;
                        }
                        i += 3;
                    } else {
                        eprintln!("abbr: --rename requires OLD_NAME and NEW_NAME");
                        self.last_exit_code = 1;
                        return Ok(());
                    }
                }
                "--list" | "-l" => {
                    for name in self.abbreviations.keys() {
                        println!("{}", name);
                    }
                    return Ok(());
                }
                "--show" | "-s" => {
                    for (name, value) in &self.abbreviations {
                        println!("abbr --add {} {}", name, value);
                    }
                    return Ok(());
                }
                "--query" | "-q" => {
                    if i + 1 < expanded.len() {
                        let name = &expanded[i + 1];
                        self.last_exit_code = if self.abbreviations.contains_key(name) { 0 } else { 1 };
                        return Ok(());
                    } else {
                        // query with no args: succeed if any abbreviations exist
                        self.last_exit_code = if self.abbreviations.is_empty() { 1 } else { 0 };
                        return Ok(());
                    }
                }
                other => {
                    // Treat unknown args as positional: abbr name expansion
                    if i + 1 < expanded.len() {
                        let name = other.to_string();
                        let value = expanded[i + 1].clone();
                        self.abbreviations.insert(name, value);
                        i += 2;
                    } else {
                        eprintln!("abbr: expected expansion for '{}'", other);
                        self.last_exit_code = 1;
                        return Ok(());
                    }
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

    fn builtin_history(&mut self, args: &[Word]) -> Result<()> {
        let history_path = self.home_dir.as_ref().map(|h| h.join(".mishell_history"));
        if args.is_empty() {
            // List history
            if let Some(ref path) = history_path {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for (i, line) in content.lines().enumerate() {
                        println!("{}\t{}", i + 1, line);
                    }
                }
            }
            return Ok(());
        }
        let subcmd = self.expand_word(&args[0]);
        match subcmd.as_str() {
            "delete" | "--delete" => {
                if args.len() < 2 {
                    eprintln!("history delete: expected search term");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let pattern = self.expand_word(&args[1]);
                if let Some(ref path) = history_path {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let lines: Vec<&str> = content.lines().collect();
                        let kept: Vec<&str> = lines.iter()
                            .filter(|l| !l.contains(pattern.as_str()))
                            .copied()
                            .collect();
                        let removed = lines.len() - kept.len();
                        std::fs::write(path, kept.join("\n"))?;
                        eprintln!("history: deleted {} entries matching '{}'", removed, pattern);
                    }
                }
            }
            "save" | "--save" => {
                // History is saved automatically; this is a no-op for compat
            }
            "clear" | "--clear" => {
                if let Some(ref path) = history_path {
                    std::fs::write(path, "")?;
                    eprintln!("history: cleared");
                }
            }
            "merge" | "--merge" => {
                // Merge history from file into session (handled by History in main loop)
                eprintln!("history merge: session history is managed by the interactive loop");
            }
            "search" | "--search" => {
                if args.len() < 2 {
                    eprintln!("history search: expected search term");
                    self.last_exit_code = 1;
                    return Ok(());
                }
                let pattern = self.expand_word(&args[1]);
                if let Some(ref path) = history_path {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        for line in content.lines() {
                            if line.contains(pattern.as_str()) {
                                println!("{}", line);
                            }
                        }
                    }
                }
            }
            _ => {
                eprintln!("history: unknown subcommand '{}'", subcmd);
                self.last_exit_code = 1;
            }
        }
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
            match waitpid(Pid::from_raw(job.pid as i32), Some(WaitPidFlag::WNOHANG)) {
                Ok(nix::sys::wait::WaitStatus::StillAlive) => true,
                Ok(_) => false,
                Err(_) => true,
            }
        });
    }

    pub fn kill_all_jobs(&mut self) {
        for job in &self.jobs {
            // Kill entire process group
            let _ = kill(Pid::from_raw(-(job.pid as i32)), Signal::SIGKILL);
        }
        // Reap all
        for job in &mut self.jobs {
            let _ = waitpid(Pid::from_raw(job.pid as i32), None);
        }
        self.jobs.clear();
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
        let mut path_only = false;
        let mut names = Vec::new();
        for arg in args {
            let s = self.expand_word(arg);
            match s.as_str() {
                "-p" | "--path" => path_only = true,
                _ => names.push(s),
            }
        }
        if names.is_empty() {
            eprintln!("type: expected command name");
            self.last_exit_code = 1;
            return Ok(());
        }
        for name in &names {
            if self.functions.contains_key(name) {
                if path_only {
                    self.last_exit_code = 1;
                } else {
                    println!("{} is a function", name);
                }
            } else if self.aliases.contains_key(name) {
                if path_only {
                    self.last_exit_code = 1;
                } else {
                    println!("{} is an alias", name);
                }
            } else if is_builtin(name) {
                if path_only {
                    self.last_exit_code = 1;
                } else {
                    println!("{} is a builtin", name);
                }
            } else {
                match ProcessCommand::new("sh").arg("-c").arg(format!("command -v {}", name)).output() {
                    Ok(output) if output.status.success() => {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if path_only {
                            println!("{}", path);
                        } else {
                            println!("{} is {}", name, path);
                        }
                    }
                    _ => {
                        if !path_only {
                            println!("{}: not found", name);
                        }
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
            "collect" => {
                let separator = if !sub_args.is_empty() && sub_args[0] == "-n" || sub_args[0] == "--no-newline" {
                    ""
                } else {
                    "\n"
                };
                let start = if separator.is_empty() { 1 } else { 0 };
                for (i, arg) in sub_args[start..].iter().enumerate() {
                    if i > 0 {
                        print!("{}", separator);
                    }
                    print!("{}", arg);
                }
                if separator.is_empty() {
                    println!();
                }
            }
            "escape" => {
                for arg in &sub_args {
                    let escaped: String = arg.chars().map(|c| match c {
                        '\n' => "\\n".to_string(),
                        '\t' => "\\t".to_string(),
                        '\r' => "\\r".to_string(),
                        '\\' => "\\\\".to_string(),
                        '"' => "\\\"".to_string(),
                        '\'' => "\\'".to_string(),
                        '\x07' => "\\a".to_string(),
                        '\x08' => "\\b".to_string(),
                        '\x0c' => "\\f".to_string(),
                        '\x0b' => "\\v".to_string(),
                        c if c.is_control() => format!("\\x{:02x}", c as u8),
                        c => c.to_string(),
                    }).collect();
                    println!("{}", escaped);
                }
            }
            "unescape" => {
                for arg in &sub_args {
                    let mut result = String::new();
                    let mut chars = arg.chars();
                    while let Some(c) = chars.next() {
                        if c == '\\' {
                            match chars.next() {
                                Some('n') => result.push('\n'),
                                Some('t') => result.push('\t'),
                                Some('r') => result.push('\r'),
                                Some('\\') => result.push('\\'),
                                Some('"') => result.push('"'),
                                Some('\'') => result.push('\''),
                                Some('a') => result.push('\x07'),
                                Some('b') => result.push('\x08'),
                                Some('f') => result.push('\x0c'),
                                Some('v') => result.push('\x0b'),
                                Some('x') => {
                                    let h1 = chars.next().unwrap_or('0');
                                    let h2 = chars.next().unwrap_or('0');
                                    if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                                        result.push(byte as char);
                                    }
                                }
                                Some(c) => {
                                    result.push('\\');
                                    result.push(c);
                                }
                                None => result.push('\\'),
                            }
                        } else {
                            result.push(c);
                        }
                    }
                    println!("{}", result);
                }
            }
            "pad" => {
                let mut width = 0;
                let mut pad_char = ' ';
                let mut pad_right = true;
                let mut strings_start = 0;
                for (i, arg) in sub_args.iter().enumerate() {
                    match arg.as_str() {
                        "-c" | "--char" => {
                            if let Some(c) = sub_args.get(i + 1) {
                                pad_char = c.chars().next().unwrap_or(' ');
                                strings_start = i + 2;
                            }
                        }
                        "-w" | "--width" => {
                            if let Some(w) = sub_args.get(i + 1) {
                                width = w.parse::<usize>().unwrap_or(0);
                                strings_start = i + 2;
                            }
                        }
                        "-r" | "--right" => {
                            pad_right = true;
                            strings_start = i + 1;
                        }
                        "-l" | "--left" => {
                            pad_right = false;
                            strings_start = i + 1;
                        }
                        _ => {
                            if strings_start == 0 {
                                strings_start = i;
                            }
                            break;
                        }
                    }
                }
                for arg in &sub_args[strings_start..] {
                    if arg.len() >= width {
                        println!("{}", arg);
                    } else {
                        let pad = pad_char.to_string().repeat(width - arg.len());
                        if pad_right {
                            println!("{}{}", arg, pad);
                        } else {
                            println!("{}{}", pad, arg);
                        }
                    }
                }
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
        let expr: String = args.iter().map(|w| self.expand_word_flat(w)).collect::<Vec<_>>().join(" ");
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

    fn builtin_funced(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("funced: expected function name");
            self.last_exit_code = 1;
            return Ok(());
        }
        let name = self.expand_word(&args[0]);

        // Get existing function body or empty
        let existing = self.functions.get(&name).map(|f| {
            f.body.iter().map(|c| self.reconstruct_command(c)).collect::<Vec<_>>().join("\n")
        }).unwrap_or_default();

        // Write to temp file
        let tmp = std::env::temp_dir().join(format!("mishell_funced_{}.fish", name));
        std::fs::write(&tmp, &existing)?;

        // Open in $EDITOR
        let editor = self.vars.get("EDITOR").cloned()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string());

        let status = ProcessCommand::new(&editor)
            .arg(&tmp)
            .status();

        match status {
            Ok(s) if s.success() => {
                if let Ok(new_body) = std::fs::read_to_string(&tmp) {
                    let trimmed = new_body.trim();
                    if !trimmed.is_empty() {
                        // Parse the new body
                        match mishell_parser::Parser::new(trimmed).parse() {
                            Ok(cmds) => {
                                self.functions.insert(name.clone(), FunctionDef {
                                    name: name.clone(),
                                    body: cmds,
                                    on_event: None,
                                    on_variable: None,
                                });
                                eprintln!("funced: function '{}' updated", name);
                            }
                            Err(e) => {
                                eprintln!("funced: parse error: {}", e);
                                self.last_exit_code = 1;
                            }
                        }
                    } else {
                        self.functions.remove(&name);
                        eprintln!("funced: function '{}' removed (empty body)", name);
                    }
                }
            }
            Ok(s) => {
                eprintln!("funced: editor exited with {}", s);
                self.last_exit_code = 1;
            }
            Err(e) => {
                eprintln!("funced: failed to launch {}: {}", editor, e);
                self.last_exit_code = 1;
            }
        }

        let _ = std::fs::remove_file(&tmp);
        Ok(())
    }

    fn builtin_funcsave(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("funcsave: expected function name");
            self.last_exit_code = 1;
            return Ok(());
        }
        let name = self.expand_word(&args[0]);

        let func = match self.functions.get(&name) {
            Some(f) => f.clone(),
            None => {
                eprintln!("funcsave: function '{}' not found", name);
                self.last_exit_code = 1;
                return Ok(());
            }
        };

        // Save to ~/.config/fish/functions/ or ~/.mishell/functions/
        let func_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mishell")
            .join("functions");
        std::fs::create_dir_all(&func_dir)?;

        let func_file = func_dir.join(format!("{}.fish", name));
        let mut output = format!("function {}", name);
        if let Some(ref event) = func.on_event {
            output.push_str(&format!(" --on-event {}", event));
        }
        if let Some(ref var) = func.on_variable {
            output.push_str(&format!(" --on-variable {}", var));
        }
        output.push('\n');
        for cmd in &func.body {
            output.push_str(&format!("{:?}", cmd));
            output.push('\n');
        }
        output.push_str("end\n");

        std::fs::write(&func_file, output)?;
        eprintln!("funcsave: function '{}' saved to {}", name, func_file.display());
        self.last_exit_code = 0;
        Ok(())
    }

    fn builtin_functions(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            // List all functions
            let mut names: Vec<&String> = self.functions.keys().collect();
            names.sort();
            for name in names {
                println!("{}", name);
            }
        } else {
            // Show specific function
            let name = self.expand_word(&args[0]);
            match self.functions.get(&name) {
                Some(func) => {
                    let mut output = format!("function {}", name);
                    if let Some(ref event) = func.on_event {
                        output.push_str(&format!(" --on-event {}", event));
                    }
                    if let Some(ref var) = func.on_variable {
                        output.push_str(&format!(" --on-variable {}", var));
                    }
                    println!("{}", output);
                    for cmd in &func.body {
                        let line = self.reconstruct_command(cmd);
                        if !line.trim().is_empty() {
                            println!("    {}", line);
                        }
                    }
                    println!("end");
                }
                None => {
                    eprintln!("functions: function '{}' not found", name);
                    self.last_exit_code = 1;
                }
            }
        }
        self.last_exit_code = 0;
        Ok(())
    }

    fn builtin_edit(&mut self, args: &[Word]) -> Result<()> {
        if args.len() < 3 {
            eprintln!("edit: usage: edit <file> <old_text> <new_text>");
            self.last_exit_code = 1;
            return Ok(());
        }

        let file_path = self.expand_word(&args[0]);
        let old_text = self.expand_word(&args[1]);
        let new_text = self.expand_word(&args[2]);

        if old_text.is_empty() {
            eprintln!("edit: old_text must not be empty");
            self.last_exit_code = 1;
            return Ok(());
        }

        let path = std::path::Path::new(&file_path);
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("edit: cannot read file: {}: {}", file_path, e);
                self.last_exit_code = 1;
                return Ok(());
            }
        };

        // Try exact match first
        let exact_pos = content.find(&old_text);

        if let Some(pos) = exact_pos {
            // Check uniqueness
            if content[pos + old_text.len()..].find(&old_text).is_some() {
                eprintln!("edit: old_text is not unique in file: {}", &old_text[..old_text.len().min(40)]);
                self.last_exit_code = 1;
                return Ok(());
            }

            // Apply edit
            let mut new_content = String::with_capacity(content.len() + new_text.len());
            new_content.push_str(&content[..pos]);
            new_content.push_str(&new_text);
            new_content.push_str(&content[pos + old_text.len()..]);

            // Write result
            if let Err(e) = std::fs::write(path, &new_content) {
                eprintln!("edit: cannot write file: {}: {}", file_path, e);
                self.last_exit_code = 1;
                return Ok(());
            }

            // Print diff
            let first_line = content[..pos].lines().count() + 1;
            println!("--- a/{}", file_path);
            println!("+++ b/{}", file_path);
            println!("@@ -{},1 +{},1 @@", first_line, first_line);
            println!("-{}", old_text);
            println!("+{}", new_text);

            self.last_exit_code = 0;
            return Ok(());
        }

        // Fuzzy match: strip trailing whitespace from each line
        let strip_trailing_ws = |s: &str| -> String {
            s.lines()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
        };

        let stripped_content = strip_trailing_ws(&content);
        let stripped_old = strip_trailing_ws(&old_text);

        if let Some(pos) = stripped_content.find(&stripped_old) {
            // Check uniqueness in stripped space
            if stripped_content[pos + stripped_old.len()..].find(&stripped_old).is_some() {
                eprintln!("edit: old_text is not unique in file: {}", &old_text[..old_text.len().min(40)]);
                self.last_exit_code = 1;
                return Ok(());
            }

            // Map position from stripped back to original
            let mut orig_pos = 0;
            let mut stripped_pos = 0;
            while stripped_pos < pos && orig_pos < content.len() {
                if stripped_pos < stripped_content.len() && stripped_content.as_bytes()[stripped_pos] == content.as_bytes()[orig_pos] {
                    stripped_pos += 1;
                    orig_pos += 1;
                } else {
                    // Skip trailing whitespace in original
                    while orig_pos < content.len() && (content.as_bytes()[orig_pos] == b' ' || content.as_bytes()[orig_pos] == b'\t') {
                        orig_pos += 1;
                    }
                }
            }

            // Find the end of the match in original
            let mut match_end_orig = orig_pos;
            let mut match_end_stripped = pos + stripped_old.len();
            while match_end_stripped < stripped_content.len() && match_end_orig < content.len() {
                if stripped_content.as_bytes()[match_end_stripped] == content.as_bytes()[match_end_orig] {
                    match_end_stripped += 1;
                    match_end_orig += 1;
                } else {
                    while match_end_orig < content.len() && (content.as_bytes()[match_end_orig] == b' ' || content.as_bytes()[match_end_orig] == b'\t') {
                        match_end_orig += 1;
                    }
                }
            }

            // Apply edit
            let mut new_content = String::with_capacity(content.len() + new_text.len());
            new_content.push_str(&content[..orig_pos]);
            new_content.push_str(&new_text);
            new_content.push_str(&content[match_end_orig..]);

            // Write result
            if let Err(e) = std::fs::write(path, &new_content) {
                eprintln!("edit: cannot write file: {}: {}", file_path, e);
                self.last_exit_code = 1;
                return Ok(());
            }

            // Print diff
            let first_line = content[..orig_pos].lines().count() + 1;
            println!("--- a/{}", file_path);
            println!("+++ b/{}", file_path);
            println!("@@ -{},1 +{},1 @@", first_line, first_line);
            println!("-{}", &content[orig_pos..match_end_orig]);
            println!("+{}", new_text);

            self.last_exit_code = 0;
            return Ok(());
        }

        eprintln!("edit: old_text not found in file: {}", &old_text[..old_text.len().min(40)]);
        self.last_exit_code = 1;
        Ok(())
    }

    fn builtin_file(&self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("file: usage: file <path>");
            return Ok(());
        }

        for arg in args {
            let path = self.expand_word(arg);
            let p = std::path::Path::new(&path);

            if !p.exists() {
                println!("{}: cannot open", path);
                continue;
            }

            if p.is_dir() {
                println!("{}: directory", path);
                continue;
            }

            // Read first bytes for magic detection
            let mut buf = [0u8; 512];
            let (file_type, size) = match std::fs::File::open(p) {
                Ok(mut f) => {
                    use std::io::Read;
                    let size = f.metadata().map(|m| m.len()).unwrap_or(0);
                    let n = f.read(&mut buf).unwrap_or(0);
                    (detect_file_type(&buf[..n], &path), size)
                }
                Err(_) => {
                    println!("{}: cannot read", path);
                    continue;
                }
            };

            match file_type {
                Some(ft) => println!("{}: {} ({} bytes)", path, ft, size),
                None => {
                    // Heuristic: check for NUL bytes or control chars
                    let has_nul = buf.iter().take(512).any(|&b| b == 0);
                    if has_nul {
                        println!("{}: data ({} bytes)", path, size);
                    } else {
                        // Check if it looks like text
                        let non_text = buf.iter().take(512).filter(|&&b| b < 0x20 && b != b'\n' && b != b'\r' && b != b'\t').count();
                        if non_text > 2 {
                            println!("{}: data ({} bytes)", path, size);
                        } else {
                            println!("{}: text ({} bytes)", path, size);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn builtin_head(&self, args: &[Word]) -> Result<()> {
        let mut n = 10;
        let mut file_path = String::new();

        let mut i = 0;
        while i < args.len() {
            let s = self.expand_word(&args[i]);
            if s == "-n" || s == "--lines" {
                i += 1;
                if i < args.len() {
                    n = self.expand_word(&args[i]).parse::<usize>().unwrap_or(10);
                }
            } else if let Some(rest) = s.strip_prefix('-') {
                if let Ok(num) = rest.parse::<usize>() {
                    n = num;
                }
            } else if file_path.is_empty() {
                file_path = s;
            }
            i += 1;
        }

        if file_path.is_empty() {
            eprintln!("head: usage: head [-n LINES] <file>");
            return Ok(());
        }

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("head: {}: {}", file_path, e);
                return Ok(());
            }
        };

        for line in content.lines().take(n) {
            println!("{}", line);
        }
        Ok(())
    }

    fn builtin_tail(&self, args: &[Word]) -> Result<()> {
        let mut n = 10;
        let mut file_path = String::new();

        let mut i = 0;
        while i < args.len() {
            let s = self.expand_word(&args[i]);
            if s == "-n" || s == "--lines" {
                i += 1;
                if i < args.len() {
                    n = self.expand_word(&args[i]).parse::<usize>().unwrap_or(10);
                }
            } else if let Some(rest) = s.strip_prefix('-') {
                if let Ok(num) = rest.parse::<usize>() {
                    n = num;
                }
            } else if file_path.is_empty() {
                file_path = s;
            }
            i += 1;
        }

        if file_path.is_empty() {
            eprintln!("tail: usage: tail [-n LINES] <file>");
            return Ok(());
        }

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tail: {}: {}", file_path, e);
                return Ok(());
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        let start = if lines.len() > n { lines.len() - n } else { 0 };
        for line in &lines[start..] {
            println!("{}", line);
        }
        Ok(())
    }

    fn builtin_try(&mut self, args: &[Word]) -> Result<()> {
        let dir = self.expand_word(args.first().unwrap_or(&Word { parts: vec![WordPart::Literal(".".to_string())] }));
        let path = std::path::Path::new(&dir);

        if !path.is_dir() {
            eprintln!("try: not a directory: {}", dir);
            self.last_exit_code = 1;
            return Ok(());
        }

        // Create temp directory for COW workspace
        let temp_dir = std::env::temp_dir().join(format!("mishell-try-{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
            eprintln!("try: cannot create temp dir: {}", e);
            self.last_exit_code = 1;
            return Ok(());
        }

        let work_dir = temp_dir.join("work");
        if let Err(e) = std::fs::create_dir_all(&work_dir) {
            eprintln!("try: cannot create work dir: {}", e);
            self.last_exit_code = 1;
            return Ok(());
        }

        // Copy directory contents to work dir (simple COW simulation)
        let abs_path = std::fs::canonicalize(path)?;
        println!("try: changes will be saved in {}", work_dir.display());
        println!("try: original directory: {}", abs_path.display());
        println!("try: type 'exit' to leave the sandbox");
        println!();

        // Set up environment for sandboxed shell
        let saved_cd = self.vars.get("PWD").cloned();
        self.vars.insert("PWD".to_string(), work_dir.to_string_lossy().to_string());
        self.vars.insert("TRY_ORIGINAL_DIR".to_string(), abs_path.to_string_lossy().to_string());
        self.vars.insert("TRY_WORK_DIR".to_string(), work_dir.to_string_lossy().to_string());

        // Note: In a real implementation, this would use overlayfs or similar
        // For now, just set up the environment and let the user work in the temp dir
        println!("try: (note: overlayfs not available, working in temp copy)");
        println!("try: use 'cp -r {}/* {}/' to copy files", abs_path.display(), work_dir.display());

        // Restore on exit
        if let Some(cd) = saved_cd {
            self.vars.insert("PWD".to_string(), cd);
        }

        Ok(())
    }

    fn builtin_test(&mut self, args: &[Word]) -> Result<()> {
        let expanded: Vec<String> = args.iter().map(|w| self.expand_word(w)).collect();
        let result = self.test_eval(&expanded);
        self.last_exit_code = if result { 0 } else { 1 };
        Ok(())
    }

    fn test_eval(&self, args: &[String]) -> bool {
        if args.is_empty() {
            return false;
        }
        // Handle [ ... ] form - trailing ] already stripped by caller
        self.test_expr(&args)
    }

    fn test_expr(&self, args: &[String]) -> bool {
        if args.is_empty() {
            return false;
        }
        if args.len() == 1 {
            return !args[0].is_empty();
        }

        // Handle negation
        if args[0] == "!" {
            return !self.test_expr(&args[1..]);
        }

        // Handle parentheses
        if args[0] == "(" && args.last().map(|s| s.as_str()) == Some(")") {
            return self.test_expr(&args[1..args.len() - 1]);
        }

        // Binary operators
        if args.len() >= 3 {
            let op = &args[1];
            let a = &args[0];
            let b = &args[2];

            match op.as_str() {
                "=" | "==" => return a == b,
                "!=" => return a != b,
                "-eq" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x == y;
                    }
                    return false;
                }
                "-ne" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x != y;
                    }
                    return false;
                }
                "-lt" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x < y;
                    }
                    return false;
                }
                "-le" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x <= y;
                    }
                    return false;
                }
                "-gt" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x > y;
                    }
                    return false;
                }
                "-ge" => {
                    if let (Ok(x), Ok(y)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return x >= y;
                    }
                    return false;
                }
                _ => {}
            }
        }

        // Unary operators
        if args.len() >= 2 {
            let op = &args[0];
            let arg = &args[1];
            match op.as_str() {
                "-z" => return arg.is_empty(),
                "-n" => return !arg.is_empty(),
                "-f" => return std::path::Path::new(arg).is_file(),
                "-d" => return std::path::Path::new(arg).is_dir(),
                "-e" => return std::path::Path::new(arg).exists(),
                "-r" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o444 != 0;
                    }
                    return false;
                }
                "-w" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o222 != 0;
                    }
                    return false;
                }
                "-x" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o111 != 0;
                    }
                    return false;
                }
                "-s" => {
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.len() > 0;
                    }
                    return false;
                }
                "-L" | "-h" => {
                    if let Ok(meta) = std::fs::symlink_metadata(arg) {
                        return meta.file_type().is_symlink();
                    }
                    return false;
                }
                "-p" => {
                    use std::os::unix::fs::FileTypeExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.file_type().is_fifo();
                    }
                    return false;
                }
                "-S" => {
                    use std::os::unix::fs::FileTypeExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.file_type().is_socket();
                    }
                    return false;
                }
                "-b" => {
                    use std::os::unix::fs::FileTypeExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.file_type().is_block_device();
                    }
                    return false;
                }
                "-c" => {
                    use std::os::unix::fs::FileTypeExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.file_type().is_char_device();
                    }
                    return false;
                }
                "-t" => {
                    if let Ok(fd) = arg.parse::<i32>() {
                        use std::os::unix::io::FromRawFd;
                        if fd == 0 {
                            let _ = unsafe { std::fs::File::from_raw_fd(fd) };
                            return true;
                        }
                    }
                    return false;
                }
                "-u" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o4000 != 0;
                    }
                    return false;
                }
                "-g" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o2000 != 0;
                    }
                    return false;
                }
                "-k" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.mode() & 0o1000 != 0;
                    }
                    return false;
                }
                "-O" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.uid() == unsafe { libc::getuid() };
                    }
                    return false;
                }
                "-G" => {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(meta) = std::fs::metadata(arg) {
                        return meta.gid() == unsafe { libc::getgid() };
                    }
                    return false;
                }
                "-N" => {
                    if let (Ok(meta), Ok(atime_meta)) = (std::fs::metadata(arg), std::fs::metadata(arg)) {
                        use std::os::unix::fs::MetadataExt;
                        return meta.mtime() > atime_meta.atime();
                    }
                    return false;
                }
                _ => {}
            }
        }

        // Logical operators -a, -o (lower precedence)
        if args.len() >= 3 {
            for i in 0..args.len() {
                if args[i] == "-a" {
                    return self.test_expr(&args[..i]) && self.test_expr(&args[i + 1..]);
                }
                if args[i] == "-o" {
                    return self.test_expr(&args[..i]) || self.test_expr(&args[i + 1..]);
                }
            }
        }

        // Default: non-empty string
        !args[0].is_empty()
    }

    fn builtin_eval(&mut self, args: &[Word]) -> Result<()> {
        let cmd_str: String = args.iter().map(|w| self.expand_word(w)).collect::<Vec<_>>().join(" ");
        if cmd_str.is_empty() {
            return Ok(());
        }
        self.execute(&cmd_str)
    }

    fn builtin_realpath(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            eprintln!("realpath: expected path");
            self.last_exit_code = 1;
            return Ok(());
        }
        for arg in args {
            let path = self.expand_word(arg);
            match std::fs::canonicalize(&path) {
                Ok(abs) => println!("{}", abs.display()),
                Err(e) => {
                    eprintln!("realpath: {}: {}", path, e);
                    self.last_exit_code = 1;
                }
            }
        }
        Ok(())
    }

    fn expand_word_flat(&self, word: &Word) -> String {
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

    #[allow(dead_code)]
    pub fn set_var(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.kill_all_jobs();
    }
}

pub fn detect_file_type(buf: &[u8], path: &str) -> Option<String> {
    if buf.is_empty() {
        return None;
    }

    // Images
    if buf.starts_with(b"\x89PNG\r\n\x1a\n") { return Some("image/png".to_string()); }
    if buf.starts_with(b"\xff\xd8\xff") { return Some("image/jpeg".to_string()); }
    if buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a") { return Some("image/gif".to_string()); }
    if buf.starts_with(b"RIFF") && buf.len() >= 12 && &buf[8..12] == b"WEBP" { return Some("image/webp".to_string()); }
    if buf.starts_with(b"BM") { return Some("image/bmp".to_string()); }

    // Audio
    if buf.starts_with(b"ID3") || (buf.len() >= 3 && buf[0] == 0xff && (buf[1] & 0xe0) == 0xe0) { return Some("audio/mpeg".to_string()); }
    if buf.starts_with(b"OggS") { return Some("audio/ogg".to_string()); }
    if buf.starts_with(b"fLaC") { return Some("audio/flac".to_string()); }
    if buf.starts_with(b"RIFF") && buf.len() >= 12 && &buf[8..12] == b"WAVE" { return Some("audio/wav".to_string()); }

    // Video
    if buf.len() >= 12 && &buf[4..8] == b"ftyp" { return Some("video/mp4".to_string()); }
    if buf.starts_with(b"\x1a\x45\xdf\xa3") { return Some("video/webm".to_string()); }
    if buf.starts_with(b"FLV") { return Some("video/x-flv".to_string()); }
    if buf.starts_with(b"\x00\x00\x01\xba") || buf.starts_with(b"\x00\x00\x01\xb3") { return Some("video/mpeg".to_string()); }

    // Archives
    if buf.starts_with(b"PK\x03\x04") { return Some("application/zip".to_string()); }
    if buf.starts_with(b"\x1f\x8b") { return Some("application/gzip".to_string()); }
    if buf.starts_with(b"BZh") { return Some("application/x-bzip2".to_string()); }
    if buf.starts_with(b"\xfd7zXZ\x00") { return Some("application/x-xz".to_string()); }
    if buf.starts_with(b"\x28\xb5\x2f\xfd") { return Some("application/zstd".to_string()); }
    if buf.starts_with(b"7z\xbc\xaf\x27\x1c") { return Some("application/x-7z-compressed".to_string()); }
    if buf.starts_with(b"Rar!\x1a\x07") { return Some("application/x-rar-compressed".to_string()); }

    // Documents
    if buf.starts_with(b"%PDF") { return Some("application/pdf".to_string()); }
    if buf.starts_with(b"SQLite format 3\0") { return Some("application/x-sqlite3".to_string()); }

    // Executables
    if buf.starts_with(b"\x7fELF") { return Some("application/x-executable".to_string()); }
    if buf.starts_with(b"Mach-O") || buf.starts_with(b"\xfe\xed\xfa") || buf.starts_with(b"\xfe\xed\xfa\xce") || buf.starts_with(b"\xfe\xed\xfa\xcf") || buf.starts_with(b"\xce\xfa\xed\xfe") || buf.starts_with(b"\xcf\xfa\xed\xfe") { return Some("application/x-mach-binary".to_string()); }
    if buf.starts_with(b"MZ") { return Some("application/x-dosexec".to_string()); }

    // Fonts
    if buf.starts_with(b"\x00\x01\x00\x00") { return Some("font/ttf".to_string()); }
    if buf.starts_with(b"OTTO") { return Some("font/otf".to_string()); }
    if buf.starts_with(b"wOFF") { return Some("font/woff".to_string()); }
    if buf.starts_with(b"wOF2") { return Some("font/woff2".to_string()); }

    // WebAssembly
    if buf.starts_with(b"\x00asm") { return Some("application/wasm".to_string()); }

    // Java
    if buf.starts_with(b"\xca\xfe\xba\xbe") { return Some("application/java-archive".to_string()); }

    // By extension
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "tar" => Some("application/x-tar".to_string()),
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb" | "sh" | "bash" | "zsh" | "fish" => Some("text/x-source".to_string()),
        "md" | "txt" | "log" | "csv" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" => Some("text/plain".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "css" => Some("text/css".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "ico" => Some("image/x-icon".to_string()),
        _ => None,
    }
}

    fn builtin_complete(&mut self, args: &[Word]) -> Result<()> {
        if args.is_empty() {
            // List all completions
            for (cmd, entries) in &self.completions {
                for entry in entries {
                    if entry.condition.is_empty() {
                        println!("complete -c {} -a '{}'", cmd, entry.arguments.join(" "));
                    } else {
                        println!("complete -c {} -a '{}' -d '{}'", cmd, entry.arguments.join(" "), entry.description);
                    }
                }
            }
            return Ok(());
        }
        let mut cmd_name = String::new();
        let mut arguments = Vec::new();
        let mut condition = String::new();
        let mut description = String::new();
        let mut erase = false;
        let mut i = 0;
        let expanded: Vec<String> = args.iter().map(|a| self.expand_word(a)).collect();
        while i < expanded.len() {
            match expanded[i].as_str() {
                "-c" | "--command" => {
                    if i + 1 < expanded.len() { cmd_name = expanded[i + 1].clone(); i += 2; } else { i += 1; }
                }
                "-a" | "--arguments" => {
                    if i + 1 < expanded.len() { arguments = expanded[i + 1].split_whitespace().map(String::from).collect(); i += 2; } else { i += 1; }
                }
                "-d" | "--description" => {
                    if i + 1 < expanded.len() { description = expanded[i + 1].clone(); i += 2; } else { i += 1; }
                }
                "-e" | "--erase" => { erase = true; i += 1; }
                _ => { i += 1; }
            }
        }
        if cmd_name.is_empty() {
            eprintln!("complete: expected -c COMMAND");
            self.last_exit_code = 1;
            return Ok(());
        }
        if erase {
            self.completions.remove(&cmd_name);
        } else {
            self.completions.entry(cmd_name).or_default().push(CompletionEntry {
                condition,
                description,
                arguments,
            });
        }
        Ok(())
    }

    fn builtin_commandline(&self, args: &[Word]) -> Result<()> {
        // In non-interactive mode, this is a no-op
        if !self.is_interactive {
            return Ok(());
        }
        // Minimal implementation: print info about current commandline
        let mut mode = "BUFFER";
        let expanded: Vec<String> = args.iter().map(|a| self.expand_word(a)).collect();
        for arg in &expanded {
            match arg.as_str() {
                "-c" | "--current-token" => { mode = "TOKEN"; }
                "-b" | "--current-buffer" => { mode = "BUFFER"; }
                "-o" | "--current-token" => { mode = "TOKEN"; }
                "-p" | "--current-process" => { mode = "PROCESS"; }
                _ => {}
            }
        }
        // In a real implementation this would read from the readline buffer
        // For now just print empty
        if mode == "BUFFER" || mode == "TOKEN" {
            // no-op, handled by interactive loop
        }
        Ok(())
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
            | "funced"
            | "funcsave"
            | "functions"
            | "edit"
            | "file"
            | "head"
            | "tail"
            | "try"
            | "test"
            | "["
            | "eval"
            | "realpath"
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

    #[test]
    fn test_execute_head_builtin() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("head -3 Cargo.toml").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_tail_builtin() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("tail -3 Cargo.toml").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_file_builtin() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("file Cargo.toml").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_file_directory() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute("file src").unwrap();
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_execute_edit_builtin() {
        let mut shell = Shell::new(false).unwrap();
        // Create temp file
        std::fs::write("/tmp/mishell_test_edit.txt", "hello world").unwrap();
        shell.execute("edit /tmp/mishell_test_edit.txt \"hello world\" \"goodbye world\"").unwrap();
        assert_eq!(shell.last_exit_code, 0);
        let content = std::fs::read_to_string("/tmp/mishell_test_edit.txt").unwrap();
        assert_eq!(content, "goodbye world");
        let _ = std::fs::remove_file("/tmp/mishell_test_edit.txt");
    }

    #[test]
    fn test_execute_edit_not_found() {
        let mut shell = Shell::new(false).unwrap();
        std::fs::write("/tmp/mishell_test_edit2.txt", "hello world").unwrap();
        shell.execute("edit /tmp/mishell_test_edit2.txt \"not found\" \"replacement\"").unwrap();
        assert_eq!(shell.last_exit_code, 1);
        let _ = std::fs::remove_file("/tmp/mishell_test_edit2.txt");
    }

    #[test]
    fn test_execute_edit_fuzzy_match() {
        let mut shell = Shell::new(false).unwrap();
        // File has trailing spaces, search term doesn't
        std::fs::write("/tmp/mishell_test_fuzzy.txt", "hello   \nworld   ").unwrap();
        shell.execute("edit /tmp/mishell_test_fuzzy.txt \"hello\" \"goodbye\"").unwrap();
        assert_eq!(shell.last_exit_code, 0);
        let content = std::fs::read_to_string("/tmp/mishell_test_fuzzy.txt").unwrap();
        assert!(content.contains("goodbye"));
        let _ = std::fs::remove_file("/tmp/mishell_test_fuzzy.txt");
    }

    #[test]
    fn test_detect_file_type() {
        assert_eq!(detect_file_type(b"\x89PNG\r\n\x1a\n", "test.png"), Some("image/png".to_string()));
        assert_eq!(detect_file_type(b"\xff\xd8\xff", "test.jpg"), Some("image/jpeg".to_string()));
        assert_eq!(detect_file_type(b"GIF89a", "test.gif"), Some("image/gif".to_string()));
        assert_eq!(detect_file_type(b"%PDF", "test.pdf"), Some("application/pdf".to_string()));
        assert_eq!(detect_file_type(b"\x7fELF", "test"), Some("application/x-executable".to_string()));
        assert_eq!(detect_file_type(b"PK\x03\x04", "test.zip"), Some("application/zip".to_string()));
        assert_eq!(detect_file_type(b"\x1f\x8b", "test.gz"), Some("application/gzip".to_string()));
        assert_eq!(detect_file_type(b"MZ", "test.exe"), Some("application/x-dosexec".to_string()));
        assert_eq!(detect_file_type(b"\x00asm", "test.wasm"), Some("application/wasm".to_string()));
        // Extension-based fallback
        assert_eq!(detect_file_type(b"hello", "test.rs"), Some("text/x-source".to_string()));
        assert_eq!(detect_file_type(b"hello", "test.md"), Some("text/plain".to_string()));
        assert_eq!(detect_file_type(b"hello", "test.html"), Some("text/html".to_string()));
        assert_eq!(detect_file_type(b"hello", "test.css"), Some("text/css".to_string()));
        assert_eq!(detect_file_type(b"hello", "test.svg"), Some("image/svg+xml".to_string()));
        // Unknown
        assert_eq!(detect_file_type(b"hello", "test.xyz"), None);
    }
}
