//! Application state machine.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::Result;

/// Application state.
#[derive(Debug)]
pub struct App {
    pub workspace: PathBuf,
    pub messages: Vec<ChatMessage>,
    pub pending_messages: Vec<String>,
    pub status_messages: Vec<String>,
    pub todo_items: Vec<TodoItem>,
    pub btw_notes: Vec<String>,
    pub input: String,
    pub cursor: usize,
    pub scroll: usize,
    pub mode: AppMode,
    pub model: String,
    pub should_quit: bool,
    pub tick_count: u64,
    pub version: String,
    pub git_branch: Option<String>,
    pub show_welcome: bool,
}

/// Todo item for task tracking.
#[derive(Debug, Clone)]
pub struct TodoItem {
    pub content: String,
    pub done: bool,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            messages: Vec::new(),
            pending_messages: Vec::new(),
            status_messages: Vec::new(),
            todo_items: Vec::new(),
            btw_notes: Vec::new(),
            input: String::new(),
            cursor: 0,
            scroll: 0,
            mode: AppMode::Normal,
            model: "gpt-4o".to_string(),
            should_quit: false,
            tick_count: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_branch: None,
            show_welcome: true,
        }
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
            // Any character starts typing immediately
            self.mode = AppMode::Input;
            self.input.insert(self.cursor, c);
            self.cursor += 1;
        }
        Ok(true)
    }

    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Enter
                if !self.input.is_empty() => {
                    self.submit_message();
                }
            KeyCode::Esc => {
                if self.input.is_empty() {
                    self.mode = AppMode::Normal;
                } else {
                    self.input.clear();
                    self.cursor = 0;
                }
            }
            KeyCode::Backspace
                if self.cursor > 0 => {
                    self.input.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
            KeyCode::Delete
                if self.cursor < self.input.len() => {
                    self.input.remove(self.cursor);
                }
            KeyCode::Left
                if self.cursor > 0 => {
                    self.cursor -= 1;
                }
            KeyCode::Right
                if self.cursor < self.input.len() => {
                    self.cursor += 1;
                }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                self.cursor = self.input.len();
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' {
                    self.should_quit = true;
                    return Ok(false);
                }
                self.input.insert(self.cursor, c);
                self.cursor += 1;
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_processing_mode(&mut self, key: KeyEvent) -> Result<bool> {
        if let KeyCode::Esc = key.code {
            // Cancel processing
            self.mode = AppMode::Normal;
        }
        Ok(true)
    }

    fn submit_message(&mut self) {
        let content = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.show_welcome = false;

        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content,
            tool_calls: Vec::new(),
            thinking: None,
        });

        // For now, add a placeholder response
        // TODO: integrate with actual LLM
        self.messages.push(ChatMessage {
            role: MessageRole::Agent,
            content: "I received your message. LLM integration pending.".to_string(),
            tool_calls: Vec::new(),
            thinking: None,
        });

        self.mode = AppMode::Normal;
    }

    pub fn handle_resize(&mut self, _width: u16, _height: u16) {
        // Handle terminal resize
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
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
}

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Input,
    Processing,
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
