//! Slash command system for the TUI.
//!
//! Parses `/command args` from user input and dispatches to handlers.

use std::collections::HashMap;
use std::path::PathBuf;

/// A slash command handler.
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: Box<dyn Fn(&str) -> SlashResult + Send + Sync>,
}

/// Result of a slash command.
#[derive(Debug, Clone)]
pub enum SlashResult {
    /// Show text to the user.
    Display(String),
    /// Switch the active model.
    SetModel(String),
    /// Start an authentication flow for a provider.
    Login(String),
    /// Force context compaction.
    Compact,
    /// Clear the conversation.
    Clear,
    /// Quit the application.
    Quit,
    /// Show session list.
    SessionList,
    /// Save current session.
    SessionSave,
    /// Load a session by id.
    SessionLoad(String),
    /// Set current session display name.
    SetSessionName(String),
    /// Show stats.
    Stats,
    /// Reload rules and skills.
    Reload,
    /// Show help.
    Help,
    /// Open theme selector.
    Theme,
    /// Add a todo item.
    TodoAdd(String),
    /// Mark a todo as done by 1-indexed number.
    TodoDone(usize),
    /// Drop a todo by 1-indexed number.
    TodoDrop(usize),
    /// Clear completed/dropped todos.
    TodoClear,
    /// List current todos.
    TodoList,
    /// Add a BTW note.
    BtwAdd(String),
    /// Clear all BTW notes.
    BtwClear,
    /// Add a task to the execution queue.
    QueueAdd(String),
    /// Show message queue status.
    QueueStatus,
    /// List loaded skills.
    SkillList,
    /// Reload skills from disk.
    SkillReload,
    /// Set thinking/reasoning level.
    SetThinkingLevel(String),
    /// Copy last assistant response to clipboard.
    Copy,
    /// Export current session to a file.
    Export(Option<String>),
    /// Resume a session (show session picker).
    Resume,
    /// Unknown command.
    Unknown(String),
}

/// Registry of slash commands.
pub struct SlashCommandRegistry {
    commands: HashMap<String, SlashCommand>,
    custom_commands: Vec<CustomCommand>,
}
impl std::fmt::Debug for SlashCommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashCommandRegistry")
            .field("commands", &self.commands.keys().collect::<Vec<_>>())
            .field("custom_commands", &self.custom_commands)
            .finish()
    }
}

/// A custom command loaded from a markdown file.
#[derive(Debug, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    pub prompt: String,
}

/// Substitute {{1}}, {{2}}, ..., {{N}} placeholders in a template with
/// whitespace-delimited arguments. {{0}} is the full args string.
/// Unmatched placeholders are left as-is.
fn substitute_template_args(template: &str, args: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut result = template.to_string();

    if result.contains("{{0}}") {
        result = result.replace("{{0}}", args);
    }

    for (i, part) in parts.iter().enumerate() {
        let placeholder = format!("{{{{{}}}}}", i + 1);
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, part);
        }
    }

    result
}

impl SlashCommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
            custom_commands: Vec::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.register(SlashCommand {
            name: "model",
            description: "Switch the active model. Usage: /model <name>",
            handler: Box::new(|args| {
                let model = args.trim();
                if model.is_empty() {
                    SlashResult::Display("Usage: /model <provider/model>".to_string())
                } else {
                    SlashResult::SetModel(model.to_string())
                }
            }),
        });

        self.register(SlashCommand {
            name: "login",
            description: "Authenticate with a provider. Usage: /login codex",
            handler: Box::new(|args| {
                let provider = args.trim();
                if provider.is_empty() {
                    SlashResult::Display("Usage: /login codex".to_string())
                } else {
                    SlashResult::Login(provider.to_string())
                }
            }),
        });

        self.register(SlashCommand {
            name: "compact",
            description: "Force context compaction",
            handler: Box::new(|_| SlashResult::Compact),
        });

        self.register(SlashCommand {
            name: "clear",
            description: "Clear the conversation",
            handler: Box::new(|_| SlashResult::Clear),
        });

        self.register(SlashCommand {
            name: "session",
            description: "Session management. Usage: /session list|save|load <id>",
            handler: Box::new(|args| {
                let parts: Vec<&str> = args.trim().split_whitespace().collect();
                match parts.first() {
                    Some(&"list") => SlashResult::SessionList,
                    Some(&"save") => SlashResult::SessionSave,
                    Some(&"load") => {
                        let id = parts.get(1).unwrap_or(&"").to_string();
                        if id.is_empty() {
                            SlashResult::Display("Usage: /session load <id>".to_string())
                        } else {
                            SlashResult::SessionLoad(id)
                        }
                    }
                    _ => SlashResult::Display("Usage: /session list|save|load <id>".to_string()),
                }
            }),
        });

        self.register(SlashCommand {
            name: "name",
            description: "Set current session display name. Usage: /name <name>",
            handler: Box::new(|args| {
                let name = args.trim();
                if name.is_empty() {
                    SlashResult::Display("Usage: /name <name>".to_string())
                } else {
                    SlashResult::SetSessionName(name.to_string())
                }
            }),
        });

        self.register(SlashCommand {
            name: "stats",
            description: "Show session statistics",
            handler: Box::new(|_| SlashResult::Stats),
        });

        self.register(SlashCommand {
            name: "reload",
            description: "Reload rules and skills from disk",
            handler: Box::new(|_| SlashResult::Reload),
        });

        self.register(SlashCommand {
            name: "help",
            description: "Show available commands",
            handler: Box::new(|_| SlashResult::Help),
        });

        self.register(SlashCommand {
            name: "hotkeys",
            description: "Show keyboard shortcuts",
            handler: Box::new(|_| SlashResult::Display(hotkeys_text())),
        });

        self.register(SlashCommand {
            name: "quit",
            description: "Exit the application",
            handler: Box::new(|_| SlashResult::Quit),
        });

        self.register(SlashCommand {
            name: "exit",
            description: "Exit the application",
            handler: Box::new(|_| SlashResult::Quit),
        });

        self.register(SlashCommand {
            name: "q",
            description: "Exit the application (alias)",
            handler: Box::new(|_| SlashResult::Quit),
        });

        self.register(SlashCommand {
            name: "theme",
            description: "Open theme selector",
            handler: Box::new(|_| SlashResult::Theme),
        });

        self.register(SlashCommand {
            name: "todo",
            description: "Task tracking. Usage: /todo add|done|drop|clear|list",
            handler: Box::new(|args| {
                let parts: Vec<&str> = args.trim().splitn(2, ' ').collect();
                match parts.first().copied().unwrap_or("") {
                    "add" => {
                        let task = parts.get(1).copied().unwrap_or("").trim();
                        if task.is_empty() {
                            SlashResult::Display("Usage: /todo add <task>".to_string())
                        } else {
                            SlashResult::TodoAdd(task.to_string())
                        }
                    }
                    "done" => match parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                        Some(n) if n > 0 => SlashResult::TodoDone(n),
                        _ => SlashResult::Display("Usage: /todo done <number>".to_string()),
                    },
                    "drop" => match parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                        Some(n) if n > 0 => SlashResult::TodoDrop(n),
                        _ => SlashResult::Display("Usage: /todo drop <number>".to_string()),
                    },
                    "clear" => SlashResult::TodoClear,
                    "list" | "" => SlashResult::TodoList,
                    _ => SlashResult::Display("Usage: /todo add|done|drop|clear|list".to_string()),
                }
            }),
        });

        self.register(SlashCommand {
            name: "btw",
            description: "BTW notes. Usage: /btw <note> or /btw clear",
            handler: Box::new(|args| {
                let args = args.trim();
                if args == "clear" {
                    SlashResult::BtwClear
                } else if args.is_empty() {
                    SlashResult::Display("Usage: /btw <note> or /btw clear".to_string())
                } else {
                    SlashResult::BtwAdd(args.to_string())
                }
            }),
        });

        self.register(SlashCommand {
            name: "queue",
            description: "Task queue. Usage: /queue <task> or /queue list",
            handler: Box::new(|args| {
                let args = args.trim();
                if args == "list" || args.is_empty() {
                    SlashResult::QueueStatus
                } else {
                    SlashResult::QueueAdd(args.to_string())
                }
            }),
        });

        self.register(SlashCommand {
            name: "think",
            description: "Set thinking level. Usage: /think off|low|medium|high",
            handler: Box::new(|args| {
                let level = args.trim().to_lowercase();
                match level.as_str() {
                    "off" | "low" | "medium" | "high" => SlashResult::SetThinkingLevel(level),
                    "" => SlashResult::Display("Usage: /think off|low|medium|high".to_string()),
                    _ => SlashResult::Display(format!(
                        "Unknown level '{}'. Use: off, low, medium, high",
                        level
                    )),
                }
            }),
        });

        self.register(SlashCommand {
            name: "copy",
            description: "Copy last assistant response to clipboard",
            handler: Box::new(|_| SlashResult::Copy),
        });
        self.register(SlashCommand {
            name: "export",
            description: "Export current session to HTML. Usage: /export [file]",
            handler: Box::new(|args| {
                let path = args.trim();
                if path.is_empty() {
                    SlashResult::Export(None)
                } else {
                    SlashResult::Export(Some(path.to_string()))
                }
            }),
        });
        self.register(SlashCommand {
            name: "resume",
            description: "Resume a different session (opens session picker)",
            handler: Box::new(|_| SlashResult::Resume),
        });
        self.register(SlashCommand {
            name: "skill",
            description: "List or reload skills. Usage: /skill list|reload",
            handler: Box::new(|args| {
                let sub = args.trim();
                match sub {
                    "reload" => SlashResult::SkillReload,
                    _ => SlashResult::SkillList,
                }
            }),
        });
    }

    fn register(&mut self, cmd: SlashCommand) {
        self.commands.insert(cmd.name.to_string(), cmd);
    }

    /// Parse and dispatch a user input that starts with '/'.
    pub fn dispatch(&self, input: &str) -> Option<SlashResult> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let input = &input[1..]; // strip the '/'
        let (cmd_name, args) = match input.find(' ') {
            Some(pos) => (&input[..pos], &input[pos + 1..]),
            None => (input, ""),
        };

        // Check built-in commands
        if let Some(cmd) = self.commands.get(cmd_name) {
            return Some((cmd.handler)(args));
        }

        // Check custom commands with {{N}} argument substitution.
        if let Some(custom) = self.custom_commands.iter().find(|c| c.name == cmd_name) {
            let expanded = substitute_template_args(&custom.prompt, args);
            return Some(SlashResult::Display(format!(
                "[Custom: {}]\n{}",
                custom.name, expanded
            )));
        }

        Some(SlashResult::Unknown(cmd_name.to_string()))
    }

    /// Load custom commands from ~/.serana/commands/*.md and .serana/commands/*.md,
    /// and prompt templates from ~/.serana/prompts/*.md and .serana/prompts/*.md.
    pub async fn load_custom_commands(&mut self) {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".serana")
            .join("commands");

        let project_dir = PathBuf::from(".serana").join("commands");

        for dir in &[home_dir, project_dir] {
            if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let name = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let description = content
                                .lines()
                                .next()
                                .unwrap_or("")
                                .trim_start_matches('#')
                                .trim()
                                .to_string();
                            self.custom_commands.push(CustomCommand {
                                name,
                                description,
                                prompt: content,
                            });
                        }
                    }
                }
            }
        }

        let home_prompts = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".serana")
            .join("prompts");
        let project_prompts = PathBuf::from(".serana").join("prompts");

        for dir in &[home_prompts, project_prompts] {
            if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        let name = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if self.custom_commands.iter().any(|c| c.name == name) {
                            continue;
                        }
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let description = content
                                .lines()
                                .next()
                                .unwrap_or("")
                                .trim_start_matches('#')
                                .trim()
                                .to_string();
                            self.custom_commands.push(CustomCommand {
                                name,
                                description,
                                prompt: content,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Get help text for all commands.
    pub fn help_text(&self) -> String {
        let mut lines = vec!["Available commands:".to_string()];

        let mut cmds: Vec<_> = self.commands.values().collect();
        cmds.sort_by_key(|c| c.name);

        for cmd in cmds {
            lines.push(format!("  /{} — {}", cmd.name, cmd.description));
        }

        for custom in &self.custom_commands {
            lines.push(format!("  /{} — {}", custom.name, custom.description));
        }

        lines.join("\n")
    }

    /// Check if input is a slash command.
    pub fn is_command(input: &str) -> bool {
        input.trim().starts_with('/')
    }
    /// List all available command names with descriptions (for autocomplete).
    pub fn list_commands(&self) -> Vec<(&str, &str)> {
        let mut cmds: Vec<_> = self
            .commands
            .values()
            .map(|c| (c.name, c.description))
            .collect();
        for custom in &self.custom_commands {
            cmds.push((&custom.name, &custom.description));
        }
        cmds.sort_by(|a, b| a.0.cmp(b.0));
        cmds
    }
}

fn hotkeys_text() -> String {
    [
        "Keyboard shortcuts:",
        "  ? - Show help overlay",
        "  Ctrl+M - Open model selector",
        "  Ctrl+T - Open theme selector",
        "  Ctrl+D / Ctrl+Q - Quit",
        "  Enter - Submit message",
        "  Shift+Enter - Insert newline",
        "  Tab - Complete slash command, model, or path",
        "  Esc - Cancel autocomplete, clear input, or return to normal mode",
        "  Up / Down - Move through autocomplete items or editor lines",
        "  Ctrl+Z - Undo editor change",
    ]
    .join("\n")
}

impl Default for SlashCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_commands() {
        let registry = SlashCommandRegistry::new();

        match registry.dispatch("/model gpt-4") {
            Some(SlashResult::SetModel(m)) => assert_eq!(m, "gpt-4"),
            _ => panic!("Expected SetModel"),
        }

        match registry.dispatch("/compact") {
            Some(SlashResult::Compact) => {}
            _ => panic!("Expected Compact"),
        }

        match registry.dispatch("/login codex") {
            Some(SlashResult::Login(provider)) => assert_eq!(provider, "codex"),
            _ => panic!("Expected Login"),
        }

        match registry.dispatch("/clear") {
            Some(SlashResult::Clear) => {}
            _ => panic!("Expected Clear"),
        }
    }

    #[test]
    fn detects_slash_commands() {
        assert!(SlashCommandRegistry::is_command("/model gpt-4"));
        assert!(SlashCommandRegistry::is_command("/help"));
        assert!(!SlashCommandRegistry::is_command("hello world"));
    }

    #[test]
    fn unknown_command() {
        let registry = SlashCommandRegistry::new();
        match registry.dispatch("/nonexistent") {
            Some(SlashResult::Unknown(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected Unknown"),
        }
    }

    #[test]
    fn not_a_command() {
        let registry = SlashCommandRegistry::new();
        assert!(registry.dispatch("hello").is_none());
    }

    #[test]
    fn help_text_lists_commands() {
        let registry = SlashCommandRegistry::new();
        let help = registry.help_text();
        assert!(help.contains("/model"));
        assert!(help.contains("/login"));
        assert!(help.contains("/compact"));
        assert!(help.contains("/hotkeys"));
        assert!(help.contains("/help"));
    }

    #[test]
    fn login_requires_provider() {
        let registry = SlashCommandRegistry::new();
        match registry.dispatch("/login") {
            Some(SlashResult::Display(message)) => assert_eq!(message, "Usage: /login codex"),
            _ => panic!("Expected Display"),
        }
    }

    #[test]
    fn hotkeys_command_shows_shortcuts() {
        let registry = SlashCommandRegistry::new();
        match registry.dispatch("/hotkeys") {
            Some(SlashResult::Display(message)) => {
                assert!(message.contains("Ctrl+M"));
                assert!(message.contains("Shift+Enter"));
            }
            _ => panic!("Expected Display"),
        }
    }

    #[test]
    fn session_subcommands() {
        let registry = SlashCommandRegistry::new();

        match registry.dispatch("/session list") {
            Some(SlashResult::SessionList) => {}
            _ => panic!("Expected SessionList"),
        }

        match registry.dispatch("/session save") {
            Some(SlashResult::SessionSave) => {}
            _ => panic!("Expected SessionSave"),
        }

        match registry.dispatch("/session load sess_123") {
            Some(SlashResult::SessionLoad(id)) => assert_eq!(id, "sess_123"),
            _ => panic!("Expected SessionLoad"),
        }
    }

    #[test]
    fn name_command_sets_session_name() {
        let registry = SlashCommandRegistry::new();
        match registry.dispatch("/name refactor pi parity") {
            Some(SlashResult::SetSessionName(name)) => assert_eq!(name, "refactor pi parity"),
            _ => panic!("Expected SetSessionName"),
        }

        match registry.dispatch("/name") {
            Some(SlashResult::Display(message)) => assert_eq!(message, "Usage: /name <name>"),
            _ => panic!("Expected Display"),
        }
    }

    #[test]
    fn export_command_accepts_optional_path() {
        let registry = SlashCommandRegistry::new();

        match registry.dispatch("/export") {
            Some(SlashResult::Export(None)) => {}
            _ => panic!("Expected Export(None)"),
        }

        match registry.dispatch("/export out/session.html") {
            Some(SlashResult::Export(Some(path))) => assert_eq!(path, "out/session.html"),
            _ => panic!("Expected Export(Some)"),
        }
    }

    #[test]
    fn template_arg_substitution() {
        assert_eq!(
            substitute_template_args("Hello {{1}} and {{2}}", "world friend"),
            "Hello world and friend"
        );
        assert_eq!(
            substitute_template_args("Echo: {{0}}", "all of this"),
            "Echo: all of this"
        );
        assert_eq!(
            substitute_template_args("Need {{1}} and {{5}}", "one"),
            "Need one and {{5}}"
        );
        assert_eq!(substitute_template_args("static text", ""), "static text");
        assert_eq!(substitute_template_args("", "args"), "");
    }
}
