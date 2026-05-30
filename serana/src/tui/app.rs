//! Application state machine.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use serana_core::Result;

use super::slash_commands::SlashCommandRegistry;
use super::symbols::{self, Symbols};

/// An autocomplete suggestion item.
#[derive(Debug, Clone)]
pub struct AutocompleteItem {
    pub value: String,
    pub description: String,
}

/// Autocomplete popup state.
#[derive(Debug, Clone)]
pub struct AutocompleteState {
    /// Filtered suggestion items.
    pub items: Vec<AutocompleteItem>,
    /// Currently selected index.
    pub selected: usize,
    /// The prefix being completed (e.g. "/qui" or "/model ").
    pub prefix: String,
}

/// Check if query is a subsequence of target (fuzzy match).
fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut qi = 0;
    let query_bytes = query.as_bytes();
    for tc in target.bytes() {
        if tc == query_bytes[qi] {
            qi += 1;
            if qi == query_bytes.len() {
                return true;
            }
        }
    }
    false
}

/// Score a fuzzy match. Higher = better.
/// exact > starts-with > contains > subsequence
fn fuzzy_score(query: &str, target: &str) -> i32 {
    if query.is_empty() {
        return 1;
    }
    if target == query {
        return 100;
    }
    if target.starts_with(query) {
        return 80;
    }
    if target.contains(query) {
        return 60;
    }
    // Subsequence: score by gaps (fewer gaps = better)
    let mut qi = 0;
    let mut gaps = 0i32;
    let mut last_match_idx: i32 = -1;
    let query_bytes = query.as_bytes();
    for (ti, tc) in target.bytes().enumerate() {
        if tc == query_bytes[qi] {
            if last_match_idx >= 0 && (ti as i32 - last_match_idx) > 1 {
                gaps += 1;
            }
            last_match_idx = ti as i32;
            qi += 1;
            if qi == query_bytes.len() {
                break;
            }
        }
    }
    if qi != query_bytes.len() {
        return 0;
    }
    (40 - gaps * 5).max(1)
}

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
    pub editor: super::editor::Editor,
    pub active_dialog: Option<super::dialog::Dialog>,
    pub scroll: usize,
    pub mode: AppMode,
    pub model: String,
    pub provider: String,
    pub should_quit: bool,
    pub tick_count: u64,
    pub version: String,
    pub git_branch: Option<String>,
    pub show_welcome: bool,
    pub show_help: bool,
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
    pub slash_commands: SlashCommandRegistry,
    pub autocomplete: Option<AutocompleteState>,
    // Session persistence
    pub session_store: Option<crate::agent::SessionStore>,
    pub current_session_id: Option<String>,
    pub recent_sessions: Vec<crate::agent::SessionMeta>,
    // Skills
    pub skill_store: Option<crate::tools::skill::SkillStore>,
    // Task queue for /queue command
    pub task_queue: VecDeque<String>,
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
            editor: super::editor::Editor::new(),
            active_dialog: None,
            scroll: 0,
            mode: AppMode::Normal,
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            should_quit: false,
            tick_count: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_branch: None,
            show_welcome: true,
            show_help: false,
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
            slash_commands: SlashCommandRegistry::new(),
            autocomplete: None,
            session_store: None,
            current_session_id: None,
            recent_sessions: Vec::new(),
            skill_store: None,
            task_queue: VecDeque::new(),
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
        // Help overlay closes on any key
        if self.show_help {
            self.show_help = false;
            return Ok(true);
        }

        // Dialog intercepts all keys
        if self.active_dialog.is_some() {
            return self.handle_dialog_key(key);
        }
        // Keyboard shortcuts to open dialogs
        if self.mode == AppMode::Input || self.mode == AppMode::Normal {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('m') => {
                        self.open_model_dialog();
                        return Ok(true);
                    }
                    KeyCode::Char('t') => {
                        self.open_theme_dialog();
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
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
            if c == '?' {
                self.show_help = true;
                return Ok(true);
            }
            self.mode = AppMode::Input;
            self.editor.insert_char(c);
        }
        Ok(true)
    }

    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        // When autocomplete is active, handle special keys first
        if self.autocomplete.is_some() {
            match key.code {
                KeyCode::Up => {
                    if let Some(ref mut ac) = self.autocomplete {
                        if ac.selected > 0 {
                            ac.selected -= 1;
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Down => {
                    if let Some(ref mut ac) = self.autocomplete {
                        if ac.selected + 1 < ac.items.len() {
                            ac.selected += 1;
                        }
                    }
                    return Ok(true);
                }
                KeyCode::Esc => {
                    self.cancel_autocomplete();
                    return Ok(true);
                }
                KeyCode::Tab => {
                    self.apply_autocomplete_selection(false);
                    return Ok(true);
                }
                KeyCode::Enter if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                    // For slash commands: apply then submit
                    if self
                        .autocomplete
                        .as_ref()
                        .map_or(false, |ac| ac.prefix.starts_with('/'))
                    {
                        self.apply_autocomplete_selection(true);
                        return Ok(true);
                    }
                    // Otherwise fall through to normal Enter handling
                }
                _ => {} // Fall through to normal handling (typing updates autocomplete)
            }
        }

        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.editor.insert_newline();
                self.cancel_autocomplete();
            }
            KeyCode::Enter if !self.editor.is_empty() => {
                self.cancel_autocomplete();
                self.submit_message();
            }
            KeyCode::Esc => {
                self.cancel_autocomplete();
                if self.editor.is_empty() {
                    self.mode = AppMode::Normal;
                } else {
                    self.editor.clear();
                }
            }
            KeyCode::Backspace => {
                self.editor.delete_backward();
                self.try_update_autocomplete();
            }
            KeyCode::Delete => {
                self.editor.delete_forward();
                self.try_update_autocomplete();
            }
            KeyCode::Left => {
                self.editor.move_left();
                self.cancel_autocomplete();
            }
            KeyCode::Right => {
                self.editor.move_right();
                self.cancel_autocomplete();
            }
            KeyCode::Up => self.editor.move_up(),
            KeyCode::Down => self.editor.move_down(),
            KeyCode::Home => self.editor.move_home(),
            KeyCode::End => self.editor.move_end(),
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.undo();
                self.cancel_autocomplete();
            }
            KeyCode::Tab => {
                if self.autocomplete.is_some() {
                    self.apply_autocomplete_selection(false);
                } else {
                    self.editor.complete_path(&self.workspace);
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' {
                    self.should_quit = true;
                    return Ok(false);
                }
                self.editor.insert_char(c);
                self.try_update_autocomplete();
            }
            _ => {}
        }
        Ok(true)
    }

    /// Try to trigger or update autocomplete based on current editor content.
    fn try_update_autocomplete(&mut self) {
        let content = self.editor.content();
        if !content.starts_with('/') {
            self.autocomplete = None;
            return;
        }

        let prefix = content.trim();
        // Strip the '/' for matching
        let query = if prefix.len() > 1 { &prefix[1..] } else { "" };

        let lower_query = query.to_lowercase();

        // Check if we're past command name (has a space) — no autocomplete for args yet
        if query.contains(' ') {
            self.autocomplete = None;
            return;
        }

        let commands = self.slash_commands.list_commands();
        let mut scored: Vec<(AutocompleteItem, i32)> = commands
            .iter()
            .filter_map(|(name, desc)| {
                let lower_name = name.to_lowercase();
                let lower_desc = desc.to_lowercase();
                if !fuzzy_match(&lower_query, &lower_name)
                    && !fuzzy_match(&lower_query, &lower_desc)
                {
                    return None;
                }
                let name_score = if fuzzy_match(&lower_query, &lower_name) {
                    fuzzy_score(&lower_query, &lower_name)
                } else {
                    0
                };
                let desc_score = if fuzzy_match(&lower_query, &lower_desc) {
                    fuzzy_score(&lower_query, &lower_desc) / 2
                } else {
                    0
                };
                Some((
                    AutocompleteItem {
                        value: name.to_string(),
                        description: desc.to_string(),
                    },
                    name_score.max(desc_score),
                ))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let items: Vec<AutocompleteItem> = scored.into_iter().map(|(item, _)| item).collect();

        if items.is_empty() {
            self.autocomplete = None;
        } else {
            // Preserve selection if possible, otherwise reset
            let selected = match &self.autocomplete {
                Some(ac) if ac.selected < items.len() => ac.selected,
                _ => 0,
            };
            self.autocomplete = Some(AutocompleteState {
                items,
                selected,
                prefix: prefix.to_string(),
            });
        }
    }

    /// Apply the currently selected autocomplete item.
    /// If `also_submit` is true, also submit the message after applying.
    fn apply_autocomplete_selection(&mut self, also_submit: bool) {
        let item = match self
            .autocomplete
            .as_ref()
            .and_then(|ac| ac.items.get(ac.selected).cloned())
        {
            Some(item) => item,
            None => return,
        };
        self.autocomplete = None;

        // Replace editor content with "/command " (with trailing space)
        self.editor.set_content(&format!("/{} ", item.value));

        if also_submit {
            self.submit_message();
        }
    }

    /// Cancel autocomplete popup.
    fn cancel_autocomplete(&mut self) {
        self.autocomplete = None;
    }

    /// Load recent sessions from the store for /session commands.
    pub fn load_recent_sessions(&mut self) {
        if let Some(ref store) = self.session_store {
            if let Ok(sessions) = store.list_recent_sessions(20) {
                self.recent_sessions = sessions;
            }
        }
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

        if let Some(result) = self.slash_commands.dispatch(&content) {
            self.handle_slash_result(result);
            return;
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

    /// Handle the result of a dispatched slash command.
    fn handle_slash_result(&mut self, result: super::slash_commands::SlashResult) {
        use super::slash_commands::SlashResult;

        match result {
            SlashResult::Display(text) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: text,
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::Quit => {
                self.should_quit = true;
            }
            SlashResult::SetModel(model) => {
                self.model = model.clone();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Model switched to: {}", model),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::Compact => {
                // Keep system prompt and last few messages, summarize the rest
                let keep_last = 4.min(self.messages.len());
                let drain_end = self.messages.len().saturating_sub(keep_last);
                if drain_end > 0 {
                    let removed = drain_end;
                    // Build a summary of removed messages
                    let summary_parts: Vec<String> = self.messages[..drain_end]
                        .iter()
                        .filter(|m| !m.content.is_empty())
                        .map(|m| {
                            let role = match m.role {
                                MessageRole::User => "User",
                                MessageRole::Agent => "Assistant",
                                MessageRole::System => "System",
                            };
                            let preview: String = m.content.chars().take(120).collect();
                            format!("{}: {}", role, preview)
                        })
                        .collect();
                    let summary = if summary_parts.is_empty() {
                        format!("Compacted {} messages.", removed)
                    } else {
                        format!(
                            "Compacted {} messages. Summary:\n{}",
                            removed,
                            summary_parts.join("\n")
                        )
                    };
                    self.messages.drain(..drain_end);
                    // Insert compaction summary as system message at the start
                    self.messages.insert(
                        0,
                        ChatMessage {
                            role: MessageRole::System,
                            content: summary,
                            tool_calls: Vec::new(),
                            thinking: None,
                        },
                    );
                    self.scroll = 0;
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Nothing to compact.".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::Clear => {
                self.messages.clear();
                self.pending_messages.clear();
                self.scroll = 0;
                self.show_welcome = true;
            }
            SlashResult::Reload => {
                // Reload skills from disk
                let workspace = self.workspace.clone();
                let new_store = crate::tools::skill::SkillStore::discover(&workspace);
                let count = new_store.len();
                self.skill_store = Some(new_store);
                // Reload custom slash commands
                self.slash_commands = SlashCommandRegistry::new();
                let rt = tokio::runtime::Handle::current();
                rt.block_on(self.slash_commands.load_custom_commands());
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Reloaded: {} skill(s), custom commands refreshed.",
                        count
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::SessionList => {
                self.load_recent_sessions();
                self.show_session_list();
            }
            SlashResult::SessionSave => {
                self.open_session_dialog();
            }
            SlashResult::SessionLoad(id) => {
                self.load_session_by_id(&id);
            }
            SlashResult::Stats => {
                let elapsed = self.session_elapsed();
                let cost = self.cost_estimate();
                let rate = self.token_rate();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Session stats:\n  Time: {}\n  Tokens in: {} / out: {} / cache-r: {} / cache-w: {}\n  Rate: {:.1} tok/s\n  Cost: ${:.4}\n  Model: {} ({})\n  Iterations: {}/{}",
                        elapsed, self.tokens_input, self.tokens_output,
                        self.tokens_cache_read, self.tokens_cache_write,
                        rate, cost, self.model, self.provider,
                        self.iterations_used, self.iterations_max,
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::Help => {
                let help = self.slash_commands.help_text();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: help,
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::Theme => {
                self.open_theme_dialog();
            }
            SlashResult::TodoAdd(task) => {
                self.todo_items.push(TodoItem {
                    content: task.clone(),
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
            SlashResult::TodoDone(n) => {
                if let Some(item) = self.todo_items.get_mut(n - 1) {
                    let task = item.content.clone();
                    item.status = TodoStatus::Done;
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("✅ Completed: {}", task),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Invalid todo number. Use 1-{}.", self.todo_items.len()),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::TodoDrop(n) => {
                if let Some(item) = self.todo_items.get_mut(n - 1) {
                    let task = item.content.clone();
                    item.status = TodoStatus::Abandoned;
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("🗑️ Dropped: {}", task),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Invalid todo number. Use 1-{}.", self.todo_items.len()),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::TodoClear => {
                let before = self.todo_items.len();
                self.todo_items.retain(|t| {
                    t.status == TodoStatus::Pending || t.status == TodoStatus::InProgress
                });
                let cleared = before - self.todo_items.len();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: if cleared > 0 {
                        format!("🧹 Cleared {} completed/dropped todos.", cleared)
                    } else {
                        "No completed todos to clear.".to_string()
                    },
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::TodoList => {
                let pending: Vec<&TodoItem> = self
                    .todo_items
                    .iter()
                    .filter(|t| {
                        t.status == TodoStatus::Pending || t.status == TodoStatus::InProgress
                    })
                    .collect();
                if pending.is_empty() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "No pending todos.".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                } else {
                    let mut lines = vec![format!("Pending todos ({}):", pending.len())];
                    for (i, todo) in pending.iter().enumerate() {
                        lines.push(format!("  {}. {}", i + 1, todo.content));
                    }
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: lines.join("\n"),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::BtwAdd(note) => {
                self.btw_notes.push(note.clone());
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("📌 BTW note added: {}", note),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::BtwClear => {
                self.btw_notes.clear();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "BTW notes cleared.".to_string(),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::QueueAdd(task) => {
                self.task_queue.push_back(task.clone());
                let pos = self.task_queue.len();
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Queued task #{}: {} ({} in queue)",
                        pos,
                        task,
                        self.task_queue.len()
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::QueueStatus => {
                let status = if self.mode == AppMode::Processing {
                    "processing"
                } else {
                    "idle"
                };
                if self.task_queue.is_empty() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Queue: empty, mode: {}", status),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                } else {
                    let mut lines = vec![format!(
                        "Queue: {} task(s), mode: {}",
                        self.task_queue.len(),
                        status
                    )];
                    for (i, task) in self.task_queue.iter().enumerate() {
                        lines.push(format!("  {}. {}", i + 1, task));
                    }
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: lines.join("\n"),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::SetThinkingLevel(level) => {
                self.thinking_level = match level.as_str() {
                    "off" => ThinkingLevel::Off,
                    "low" => ThinkingLevel::Low,
                    "medium" => ThinkingLevel::Medium,
                    "high" => ThinkingLevel::High,
                    _ => ThinkingLevel::Off,
                };
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Thinking level set to: {}", level),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            SlashResult::Copy => {
                let last_assistant = self
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::Agent);
                match last_assistant {
                    Some(msg) => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "📋 Last response ({} chars) — clipboard copy not yet wired",
                                msg.content.len()
                            ),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                    None => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: "No assistant response to copy.".to_string(),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                }
            }
            SlashResult::SkillList => {
                if let Some(ref store) = self.skill_store {
                    let skills = store.all();
                    if skills.is_empty() {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: "No skills loaded. Create one with skill_create tool or add SKILL.md files to .serana/skills/.".to_string(),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    } else {
                        let mut lines = vec![format!("Loaded skills ({}):", skills.len())];
                        let mut sorted: Vec<_> = skills;
                        sorted.sort_by(|a, b| a.name.cmp(&b.name));
                        for s in &sorted {
                            lines.push(format!(
                                "  /skill:{} — {} ({})",
                                s.name, s.description, s.source
                            ));
                        }
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: lines.join("\n"),
                            tool_calls: Vec::new(),
                            thinking: None,
                        });
                    }
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Skill store not initialized.".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::SkillReload => {
                if let Some(ref mut store) = self.skill_store {
                    *store = crate::tools::skill::SkillStore::discover(&self.workspace);
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Skills reloaded. {} skill(s) found.", store.len()),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
            }
            SlashResult::Resume => {
                self.load_recent_sessions();
                self.active_dialog = Some(super::dialog::Dialog {
                    kind: super::dialog::DialogKind::SessionResume,
                    title: " Resume Session ".to_string(),
                    items: if self.recent_sessions.is_empty() {
                        vec![super::dialog::DialogItem {
                            label: "No saved sessions found".into(),
                            description: String::new(),
                            value: String::new(),
                        }]
                    } else {
                        self.recent_sessions
                            .iter()
                            .map(|s| {
                                let label = s.title.as_deref().unwrap_or(&s.id);
                                let date = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
                                super::dialog::DialogItem {
                                    label: format!("{} — {} msgs, {}", label, s.message_count, date),
                                    description: s.id.clone(),
                                    value: s.id.clone(),
                                }
                            })
                            .collect()
                    },
                    selected: Some(0),
                    filter: String::new(),
                });
            }
            SlashResult::Unknown(name) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Unknown command: /{}. Type /help for available commands.",
                        name
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
        }
    }

    /// Handle keys when a dialog is open.
    fn handle_dialog_key(&mut self, key: KeyEvent) -> Result<bool> {
        let dialog = self.active_dialog.as_mut().unwrap();
        match key.code {
            KeyCode::Esc => {
                self.active_dialog = None;
            }
            KeyCode::Enter => {
                if let Some(value) = dialog.selected_value() {
                    let value = value.to_string();
                    let kind = dialog.kind;
                    self.active_dialog = None;
                    match kind {
                        super::dialog::DialogKind::ModelSelector => {
                            self.model = value;
                            self.messages.push(ChatMessage {
                                role: MessageRole::System,
                                content: format!("Model switched to: {}", self.model),
                                tool_calls: Vec::new(),
                                thinking: None,
                            });
                        }
                        super::dialog::DialogKind::ThemeSelector => {
                            self.messages.push(ChatMessage {
                                role: MessageRole::System,
                                content: format!("Theme selected: {}", value),
                                tool_calls: Vec::new(),
                                thinking: None,
                            });
                        }
                        super::dialog::DialogKind::SessionSelector => {
                            self.messages.push(ChatMessage {
                                role: MessageRole::System,
                                content: format!("Session selected: {}", value),
                                tool_calls: Vec::new(),
                                thinking: None,
                            });
                        }
                        super::dialog::DialogKind::SessionResume => {
                            if !value.is_empty() {
                                self.load_session_by_id(&value);
                            }
                        }
                    }
                }
            }
            KeyCode::Up => {
                dialog.select_previous();
            }
            KeyCode::Down => {
                dialog.select_next();
            }
            KeyCode::Backspace => {
                dialog.filter_backspace();
            }
            KeyCode::Char(c) => {
                dialog.filter_input(c);
            }
            _ => {}
        }
        Ok(true)
    }

    /// Open the model selector dialog.
    pub fn open_model_dialog(&mut self) {
        let models = vec![
            ("gpt-4o".into(), "OpenAI flagship".into()),
            ("gpt-4o-mini".into(), "OpenAI fast".into()),
            ("claude-3.5-sonnet".into(), "Anthropic".into()),
            ("claude-3-haiku".into(), "Anthropic fast".into()),
            ("deepseek-coder".into(), "DeepSeek".into()),
            ("codellama-70b".into(), "Meta".into()),
        ];
        self.active_dialog = Some(super::dialog::Dialog::new_model_selector(models));
    }

    /// Open the theme selector dialog.
    pub fn open_theme_dialog(&mut self) {
        let themes = vec![
            ("default".into(), "Oceanic dark".into()),
            ("light".into(), "Light mode".into()),
            ("colorblind".into(), "Colorblind friendly".into()),
        ];
        self.active_dialog = Some(super::dialog::Dialog::new_theme_selector(themes));
    }

    /// Open the session selector dialog.
    pub fn open_session_dialog(&mut self) {
        let mut sessions: Vec<(String, String)> = self
            .recent_sessions
            .iter()
            .map(|s| {
                let label = s.title.as_deref().unwrap_or(&s.id);
                let date = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
                let desc = format!("{} — {} msgs", date, s.message_count);
                (format!("{} — {}", label, desc), s.id.clone())
            })
            .collect();
        if sessions.is_empty() {
            sessions.push(("current".into(), "Current session".into()));
        }
        self.active_dialog = Some(super::dialog::Dialog::new_session_selector(sessions));
    }

    /// Show recent sessions as a system message list.
    fn show_session_list(&mut self) {
        if self.recent_sessions.is_empty() {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: "No saved sessions found.".to_string(),
                tool_calls: Vec::new(),
                thinking: None,
            });
            return;
        }
        let current = self.current_session_id.as_deref().unwrap_or("");
        let mut lines = vec![format!("Recent sessions ({}):", self.recent_sessions.len())];
        for (i, s) in self.recent_sessions.iter().take(10).enumerate() {
            let marker = if s.id == current { " ← current" } else { "" };
            let title = s.title.as_deref().unwrap_or(&s.id);
            let date = s.updated_at.format("%Y-%m-%d %H:%M");
            lines.push(format!(
                "  {}. {} — {} msgs, {}{}",
                i + 1,
                title,
                s.message_count,
                date,
                marker,
            ));
        }
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: lines.join("\n"),
            tool_calls: Vec::new(),
            thinking: None,
        });
    }

    /// Load a session by id and display it.
    fn load_session_by_id(&mut self, id: &str) {
        let store = match self.session_store.as_ref() {
            Some(s) => s,
            None => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "No session store available.".to_string(),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
                return;
            }
        };
        match store.load_session(id) {
            Ok(Some(session)) => {
                self.current_session_id = Some(id.to_string());
                // Restore messages from the session
                self.messages.clear();
                for msg in &session.messages {
                    let role = match msg.role.as_str() {
                        "user" => MessageRole::User,
                        "assistant" => MessageRole::Agent,
                        _ => MessageRole::System,
                    };
                    self.messages.push(ChatMessage {
                        role,
                        content: msg.content.clone(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }
                let title = session.meta.title.as_deref().unwrap_or(id);
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Loaded session '{}' ({} messages)",
                        title,
                        session.messages.len(),
                    ),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
                self.show_welcome = false;
            }
            Ok(None) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Session '{}' not found.", id),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
            Err(e) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Failed to load session: {}", e),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
        }
    }
    pub fn handle_resize(&mut self, _width: u16, _height: u16) {}

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

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            AppMode::Normal => "NORMAL",
            AppMode::Input => "INPUT",
            AppMode::Processing => "PROCESSING",
        }
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
