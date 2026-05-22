//! Application state machine.

use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use serana_core::Result;

use crate::symbols::{self, Symbols};

/// Todo status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
    Abandoned,
}

/// BTW entry (question + answer).
#[derive(Debug, Clone)]
pub struct BtwEntry {
    pub question: String,
    pub answer: Option<String>,
    pub state: BtwState,
}

impl BtwEntry {
    pub fn new(question: String) -> Self {
        Self {
            question,
            answer: None,
            state: BtwState::Pending,
        }
    }
}

/// BTW query state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtwState {
    Pending,
    Responding,
    Complete,
}

/// Application state.
#[derive(Debug)]
pub struct App {
    pub symbols: &'static Symbols,
    pub workspace: PathBuf,
    pub messages: Vec<ChatMessage>,
    pub pending_messages: Vec<String>,
    pub status_messages: Vec<String>,
    pub todo_items: Vec<TodoItem>,
    pub todo_reminder_count: u32,
    pub btw_entries: Vec<BtwEntry>,
    pub btw_notes: Vec<String>,
    pub editor: crate::editor::Editor,
    pub scroll: usize,
    pub mode: AppMode,
    pub model: String,
    pub provider: String,
    pub should_quit: bool,
    pub tick_count: u64,
    pub version: String,
    pub git_branch: Option<String>,
    pub show_welcome: bool,
    // Token usage tracking (oh-my-pi style)
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub tokens_cache_read: u64,
    pub tokens_cache_write: u64,
    // Context window tracking
    pub context_window: u64,
    // Git status counts
    pub git_staged: u32,
    pub git_unstaged: u32,
    pub git_untracked: u32,
    pub iterations_used: u64,
    pub iterations_max: u64,
    // Session tracking
    pub session_start: Instant,
    pub hostname: String,
    pub thinking_level: ThinkingLevel,
    // Cost tracking ($/M tokens)
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    // Status line preset
    pub status_preset: String,
}

/// Todo item for task tracking.
#[derive(Debug, Clone)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            symbols: &symbols::UNICODE,
            workspace,
            messages: Vec::new(),
            pending_messages: Vec::new(),
            status_messages: Vec::new(),
            todo_items: Vec::new(),
            todo_reminder_count: 0,
            btw_entries: Vec::new(),
            btw_notes: Vec::new(),
            editor: crate::editor::Editor::new(),
            scroll: 0,
            mode: AppMode::Normal,
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            should_quit: false,
            tick_count: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_branch: None,
            show_welcome: true,
            tokens_input: 0,
            tokens_output: 0,
            tokens_cache_read: 0,
            tokens_cache_write: 0,
            context_window: 200_000,
            git_staged: 0,
            git_unstaged: 0,
            git_untracked: 0,
            iterations_used: 0,
            iterations_max: 30,
            session_start: Instant::now(),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| {
                std::fs::read_to_string("/etc/hostname")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "localhost".to_string())
            }),
            thinking_level: ThinkingLevel::Off,
            input_cost_per_mtok: 2.50,
            output_cost_per_mtok: 10.00,
            status_preset: "default".to_string(),
        }
    }

    /// Create App with model from config
    pub fn with_model(workspace: PathBuf, model: String, provider: String) -> Self {
        let mut app = Self::new(workspace);
        app.model = model;
        app.provider = provider;
        app
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::Input => self.handle_input_mode(key),
            AppMode::Processing => self.handle_processing_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<bool> {
        if let KeyCode::Char(c) = key.code {
            if key.modifiers.contains(KeyModifiers::CONTROL) && (c == 'q' || c == 'd') {
                self.should_quit = true;
                return Ok(false);
            }
            self.mode = AppMode::Input;
            self.editor.insert_char(c);
        }
        Ok(true)
    }

    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.editor.insert_newline();
            }
            KeyCode::Enter if !self.editor.is_empty() => {
                self.submit_message();
            }
            KeyCode::Esc => {
                if self.editor.is_empty() {
                    self.mode = AppMode::Normal;
                } else {
                    self.editor.clear();
                }
            }
            KeyCode::Backspace => {
                self.editor.delete_backward();
            }
            KeyCode::Delete => {
                self.editor.delete_forward();
            }
            KeyCode::Left => self.editor.move_left(),
            KeyCode::Right => self.editor.move_right(),
            KeyCode::Up => self.editor.move_up(),
            KeyCode::Down => self.editor.move_down(),
            KeyCode::Home => self.editor.move_home(),
            KeyCode::End => self.editor.move_end(),
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.undo();
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' {
                    self.should_quit = true;
                    return Ok(false);
                }
                self.editor.insert_char(c);
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_processing_mode(&mut self, key: KeyEvent) -> Result<bool> {
        if let KeyCode::Esc = key.code {
            self.mode = AppMode::Normal;
        }
        Ok(true)
    }

    fn submit_message(&mut self) {
        let content = self.editor.content();
        self.editor.clear();

        if content.starts_with('/') {
            let cmd = content.trim();
            match cmd {
                "/quit" | "/exit" | "/q" => {
                    self.should_quit = true;
                    return;
                }
                "/help" | "/?" => {
                    let help = concat!(
                        "Available commands:\n",
                        "  /quit, /exit, /q - Exit\n",
                        "  /help, /? - Show this help\n",
                        "  /todo add <task> - Add a todo item\n",
                        "  /todo done <n> - Mark todo #n as done\n",
                        "  /todo drop <n> - Drop todo #n\n",
                        "  /todo clear - Clear completed todos\n",
                        "  /btw <note> - Add a BTW note\n",
                        "  /btw clear - Clear all BTW notes",
                    );
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: help.to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
                _ if cmd.starts_with("/todo ") => {
                    self.handle_todo_cmd(cmd);
                    return;
                }
                _ if cmd.starts_with("/btw ") => {
                    let note = cmd.trim_start_matches("/btw ");
                    self.btw_notes.push(note.to_string());
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("📌 BTW note added: {}", note),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
                "/btw clear" | "/btwc" => {
                    self.btw_notes.clear();
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "BTW notes cleared.".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
                _ if cmd.starts_with("/btw") => {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /btw <note> or /btw clear".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
                _ => {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!(
                            "Unknown command: {}. Type /help for available commands.",
                            cmd
                        ),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
            }
        }

        self.show_welcome = false;

        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content,
            tool_calls: Vec::new(),
            thinking: None,
        });

        self.mode = AppMode::Processing;
    }

    fn handle_todo_cmd(&mut self, cmd: &str) {
        let rest = cmd.strip_prefix("/todo ").unwrap_or("");
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let sub = parts.first().copied().unwrap_or("");

        match sub {
            "add" => {
                let task = parts.get(1).copied().unwrap_or("").trim();
                if task.is_empty() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Usage: /todo add <task description>".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                    return;
                }
                self.todo_items.push(TodoItem {
                    content: task.to_string(),
                    status: TodoStatus::Pending,
                });
                self.todo_reminder_count = 0;
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("✅ Todo added: {}", task),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            "done" => {
                let idx = parts
                    .get(1)
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .and_then(|n| n.checked_sub(1));
                match idx {
                    Some(i) if i < self.todo_items.len() => {
                        let task = self.todo_items[i].content.clone();
                        self.todo_items[i].status = TodoStatus::Done;
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("✅ Completed: {}", task),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                    _ => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "Invalid todo number. Use 1-{}.",
                                self.todo_items.len()
                            ),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                }
            }
            "drop" => {
                let idx = parts
                    .get(1)
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .and_then(|n| n.checked_sub(1));
                match idx {
                    Some(i) if i < self.todo_items.len() => {
                        let task = self.todo_items[i].content.clone();
                        self.todo_items[i].status = TodoStatus::Abandoned;
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("🗑️ Dropped: {}", task),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                    _ => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "Invalid todo number. Use 1-{}.",
                                self.todo_items.len()
                            ),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                }
            }
            "clear" => {
                let before = self.todo_items.len();
                self.todo_items
                    .retain(|t| t.status == TodoStatus::Pending || t.status == TodoStatus::InProgress);
                let cleared = before - self.todo_items.len();
                if cleared > 0 {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("🧹 Cleared {} completed/dropped todos.", cleared),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "No completed todos to clear.".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            _ => {
                let count = self.todo_items.len();
                let done = self
                    .todo_items
                    .iter()
                    .filter(|t| t.status == TodoStatus::Done || t.status == TodoStatus::Abandoned)
                    .count();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Usage: /todo add|done|drop|clear\nPending: {}/{} todos.",
                        count - done,
                        count
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
        }
    }

    pub fn handle_resize(&mut self, _width: u16, _height: u16) {
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    /// Elapsed session time as mm:ss or hh:mm:ss.
    pub fn session_elapsed(&self) -> String {
        let secs = self.session_start.elapsed().as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        if h > 0 {
            format!("{}:{:02}:{:02}", h, m, s)
        } else {
            format!("{}:{:02}", m, s)
        }
    }

    /// Estimated cost in dollars based on token usage and per-model rates.
    pub fn cost_estimate(&self) -> f64 {
        let input_cost = (self.tokens_input + self.tokens_cache_read) as f64
            * self.input_cost_per_mtok
            / 1_000_000.0;
        let output_cost = self.tokens_output as f64 * self.output_cost_per_mtok / 1_000_000.0;
        input_cost + output_cost
    }

    /// Token output rate (tokens/sec) over the session lifetime.
    pub fn token_rate(&self) -> f64 {
        let elapsed = self.session_start.elapsed().as_secs_f64();
        if elapsed < 1.0 {
            return 0.0;
        }
        self.tokens_output as f64 / elapsed
    }

    /// Progress bar string for percentage-based metrics (context, iterations).
    pub fn progress_bar(pct: u32, width: usize) -> String {
        let filled = (pct as usize * width / 100).min(width);
        let empty = width - filled;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }

    pub fn add_status(&mut self, msg: impl Into<String>) {
        self.status_messages.push(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status_messages.clear();
    }

    pub fn add_btw(&mut self, note: impl Into<String>) {
        self.btw_notes.push(note.into());
    }

    /// Set streaming response content (real-time display)
    pub fn set_pending_response(&mut self, content: String) {
        if self.pending_messages.is_empty() {
            self.pending_messages.push(content);
        } else {
            self.pending_messages[0] = content;
        }
    }

    /// Clear streaming response content
    pub fn clear_pending_response(&mut self) {
        self.pending_messages.clear();
    }

    /// Generate a todo reminder message if there are incomplete todos
    pub fn todo_reminder_message(&mut self) -> Option<String> {
        let incomplete: Vec<&TodoItem> = self
            .todo_items
            .iter()
            .filter(|t| t.status == TodoStatus::Pending || t.status == TodoStatus::InProgress)
            .collect();
        if incomplete.is_empty() {
            return None;
        }
        self.todo_reminder_count += 1;
        let max_reminders = 3;
        if self.todo_reminder_count > max_reminders {
            return None;
        }
        let count = incomplete.len();
        let label = if count == 1 { "todo" } else { "todos" };
        let mut msg = format!(
            "⚠️ {} incomplete {} (reminder {}/{})",
            count, label, self.todo_reminder_count, max_reminders
        );
        for (i, todo) in incomplete.iter().enumerate() {
            msg.push_str(&format!("\n  {}. {}", i + 1, todo.content));
        }
        Some(msg)
    }
}

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Input,
    Processing,
}

/// Thinking/reasoning level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel {
    Off,
    Low,
    Medium,
    High,
}

/// Chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub thinking: Option<String>,
}

/// Tool call representation.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub args: String,
    pub result: Option<String>,
    pub status: ToolCallStatus,
    pub diff_preview: Option<(String, String)>, // (path, diff)
}

/// Tool call status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Success,
    Error,
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Agent,
    System,
}
