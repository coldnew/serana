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
#[serde(untagged)]
pub enum Message {
    Text {
        role: String,
        content: String,
    },
    ToolCall {
        role: String,
        content: Option<String>,
        tool_calls: Vec<ToolCallData>,
    },
    ToolResult {
        role: String,
        tool_call_id: String,
        content: String,
    },
}

impl Message {
    /// Create a system message
    pub fn system(content: String) -> Self {
        Self::Text { role: "system".to_string(), content }
    }

    /// Create a user message
    pub fn user(content: String) -> Self {
        Self::Text { role: "user".to_string(), content }
    }

    /// Create an assistant message
    pub fn assistant(content: String) -> Self {
        Self::Text { role: "assistant".to_string(), content }
    }

    /// Create a tool-result message
    pub fn tool_result(tool_call_id: String, content: String) -> Self {
        Self::ToolResult { role: "tool".to_string(), tool_call_id, content }
    }
}

/// Tool call data from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallData {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, messages: &[Message]) -> Result<String>;

    /// Send a chat completion request with tool support
    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message>;
}

/// Provider type for configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    OpenAI,
    Custom { url: String },
}

