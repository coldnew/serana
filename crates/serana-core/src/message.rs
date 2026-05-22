use serde::{Deserialize, Serialize};

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
        Self::Text {
            role: "system".to_string(),
            content,
        }
    }

    /// Create a user message
    pub fn user(content: String) -> Self {
        Self::Text {
            role: "user".to_string(),
            content,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: String) -> Self {
        Self::Text {
            role: "assistant".to_string(),
            content,
        }
    }

    /// Create a tool-result message
    pub fn tool_result(tool_call_id: String, content: String) -> Self {
        Self::ToolResult {
            role: "tool".to_string(),
            tool_call_id,
            content,
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCallData>) -> Self {
        Self::ToolCall {
            role: "assistant".to_string(),
            content: None,
            tool_calls,
        }
    }

    /// Get the role of the message.
    pub fn role(&self) -> String {
        match self {
            Self::Text { role, .. } => role.clone(),
            Self::ToolCall { role, .. } => role.clone(),
            Self::ToolResult { role, .. } => role.clone(),
        }
    }

    /// Whether this message contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        matches!(self, Self::ToolCall { tool_calls, .. } if !tool_calls.is_empty())
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
