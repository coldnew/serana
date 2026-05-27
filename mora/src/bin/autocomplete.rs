use crate::shell::Shell;
use std::path::PathBuf;
use std::process::Command;

pub struct AutoCompleter {
    commands: Vec<String>,
}

impl AutoCompleter {
    pub fn new() -> Self {
        Self {
            commands: Self::load_commands(),
        }
    }

    fn load_commands() -> Vec<String> {
        let mut commands = Vec::new();

        // Load from PATH
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            commands.push(name.to_string());
                        }
                    }
                }
            }
        }

        // Add builtins
        commands.extend([
            "cd", "export", "alias", "abbr", "set", "exit",
            "pushd", "popd", "dirs", "history", "echo", "printf",
            "read", "test", "[", "true", "false", "pwd", "type",
            "hash", "help", "source", ".", "eval", "exec",
        ].iter().map(|s| s.to_string()));

        commands.sort();
        commands.dedup();
        commands
    }

    pub fn complete(&self, input: &str, cursor_pos: usize, shell: &Shell) -> Vec<String> {
        let before_cursor = &input[..cursor_pos];
        let parts: Vec<&str> = before_cursor.split_whitespace().collect();

        if parts.is_empty() || (parts.len() == 1 && !before_cursor.ends_with(' ')) {
            // Complete command name
            let prefix = parts.first().unwrap_or(&"");
            return self.complete_command(prefix);
        }

        // Complete arguments (file paths, variables, etc.)
        let last_word = parts.last().unwrap_or(&"");
        self.complete_argument(last_word, shell)
    }

    fn complete_command(&self, prefix: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .take(20)
            .cloned()
            .collect()
    }

    fn complete_argument(&self, prefix: &str, shell: &Shell) -> Vec<String> {
        let mut completions = Vec::new();

        // Variable completion
        if prefix.starts_with('$') {
            let var_prefix = &prefix[1..];
            for key in shell.vars().keys() {
                if key.starts_with(var_prefix) {
                    completions.push(format!("${}", key));
                }
            }
            return completions;
        }

        // Alias completion
        if prefix.starts_with('!') {
            let alias_prefix = &prefix[1..];
            for name in shell.aliases().keys() {
                if name.starts_with(alias_prefix) {
                    completions.push(format!("!{}", name));
                }
            }
            return completions;
        }

        // File path completion
        let (dir, file_prefix) = if let Some(pos) = prefix.rfind('/') {
            let dir = &prefix[..pos];
            let file = &prefix[pos + 1..];
            (if dir.is_empty() { "/" } else { dir }, file)
        } else {
            (".", prefix)
        };

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_prefix) && !name.starts_with('.') {
                        let path = format!("{}/{}", dir, name);
                        let path = path.replace("//", "/");
                        if entry.path().is_dir() {
                            completions.push(format!("{}/", path));
                        } else {
                            completions.push(path);
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.truncate(20);
        completions
    }
}
