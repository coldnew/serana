//! Application state machine for the TUI

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::agent::coding::CodingAgent;
use crate::config::Config;
use crate::llm::openai::OpenAiClient;
use crate::llm::LlmClient;
use crate::tools::ToolRegistry;
use crate::Result;

/// Application state
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Chat message history
    pub messages: Vec<ChatMessage>,
    /// Current input buffer
    pub input: String,
    /// Cursor position in input
    pub input_cursor: usize,
    /// Workspace path
    pub workspace: PathBuf,
    /// Files in context
    pub context_files: Vec<PathBuf>,
    /// Status message
    pub status: String,
    /// Current model name
    pub model: String,
    /// Token count (approximate)
    pub token_count: usize,
    /// Agent instance
    #[allow(dead_code)]
    agent: CodingAgent,
    /// Should quit
    pub should_quit: bool,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal mode - browsing
    Normal,
    /// Input mode - typing message
    Input,
    /// Processing - waiting for LLM
    Processing,
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        let config = Config::default();
        let llm: Box<dyn LlmClient> = Box::new(OpenAiClient::new(config.llm.clone()));
        let tools = ToolRegistry::new();
        let agent = CodingAgent::new(llm, tools);

        Self {
            mode: AppMode::Normal,
            messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            workspace: workspace.clone(),
            context_files: Vec::new(),
            status: "Ready".to_string(),
            model: config.llm.model,
            token_count: 0,
            agent,
            should_quit: false,
        }
    }

    /// Handle keyboard event. Returns false if should quit.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::Input => self.handle_input_mode(key),
            AppMode::Processing => self.handle_processing_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                Ok(false)
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = AppMode::Input;
                Ok(true)
            }
            KeyCode::Char(':') => {
                // Command mode (future)
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                Ok(true)
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.submit_input()?;
                }
                Ok(true)
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    self.mode = AppMode::Normal;
                    return Ok(true);
                }
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
                Ok(true)
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input.remove(self.input_cursor);
                }
                Ok(true)
            }
            KeyCode::Delete => {
                if self.input_cursor < self.input.len() {
                    self.input.remove(self.input_cursor);
                }
                Ok(true)
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
                Ok(true)
            }
            KeyCode::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
                Ok(true)
            }
            KeyCode::Home => {
                self.input_cursor = 0;
                Ok(true)
            }
            KeyCode::End => {
                self.input_cursor = self.input.len();
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn handle_processing_mode(&mut self, key: KeyEvent) -> Result<bool> {
        // Allow escape to cancel processing
        if key.code == KeyCode::Esc {
            self.mode = AppMode::Normal;
            self.status = "Cancelled".to_string();
        }
        Ok(true)
    }

    fn submit_input(&mut self) -> Result<()> {
        let content = std::mem::take(&mut self.input);
        self.input_cursor = 0;

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            tool_calls: Vec::new(),
        });

        // Switch to processing mode
        self.mode = AppMode::Processing;
        self.status = "Processing...".to_string();

        // TODO: Actually call the agent (async in sync context requires refactoring)
        // For now, just echo back
        self.messages.push(ChatMessage {
            role: MessageRole::Agent,
            content: format!("Received: {}", content),
            tool_calls: Vec::new(),
        });

        self.mode = AppMode::Normal;
        self.status = "Ready".to_string();

        Ok(())
    }

    /// Handle terminal resize
    pub fn handle_resize(&mut self, _width: u16, _height: u16) {
        // No special handling needed - ratatui handles it
    }

    /// Called on tick interval
    pub fn tick(&mut self) {
        // Update animations, check for async events, etc.
    }
}
