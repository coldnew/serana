//! LLM (Large Language Model) client abstraction
//!
//! Provides a trait-based interface for LLM providers with built-in
//! OpenAI-compatible support.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

pub mod openai;

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    /// Create a system message
    pub fn system(content: String) -> Self {
        Self { role: "system".to_string(), content }
    }

    /// Create a user message
    pub fn user(content: String) -> Self {
        Self { role: "user".to_string(), content }
    }

    /// Create an assistant message
    pub fn assistant(content: String) -> Self {
        Self { role: "assistant".to_string(), content }
    }
}

/// LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, messages: &[Message]) -> Result<String>;
}

/// Provider type for configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    OpenAI,
    Custom { url: String },
}

