//! Application state machine.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::Result;

/// Application state.
#[derive(Debug)]
pub struct App {
    pub workspace: PathBuf,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub cursor: usize,
    pub scroll: usize,
    pub mode: AppMode,
    pub model: String,
    pub should_quit: bool,
    pub tick_count: u64,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            messages: Vec::new(),
            input: String::new(),
            cursor: 0,
            scroll: 0,
            mode: AppMode::Normal,
            model: "gpt-4o".to_string(),
            should_quit: false,
            tick_count: 0,
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
        match key.code {
            KeyCode::Char('i') => {
                self.mode = AppMode::Input;
            }
            KeyCode::Char('q') | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(false);
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.submit_message();
                }
            }
            KeyCode::Esc => {
                if self.input.is_empty() {
                    self.mode = AppMode::Normal;
                } else {
                    self.input.clear();
                    self.cursor = 0;
                }
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.input.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
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

        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content,
            tool_calls: Vec::new(),
        });

        // For now, add a placeholder response
        // TODO: integrate with actual LLM
        self.messages.push(ChatMessage {
            role: MessageRole::Agent,
            content: "I received your message. LLM integration pending.".to_string(),
            tool_calls: Vec::new(),
        });

        self.mode = AppMode::Normal;
    }

    pub fn handle_resize(&mut self, _width: u16, _height: u16) {
        // Handle terminal resize
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
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
    pub tool_calls: Vec<String>,
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Agent,
    System,
}
