use serde_json::Value;

use crate::Result;
use async_trait::async_trait;

/// Core agent trait.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Name of the agent
    fn name(&self) -> &'static str;

    /// Execute a task given natural language instruction
    async fn execute(&self, instruction: &str) -> Result<AgentOutput>;

    /// Process a message and return response (for interactive mode)
    async fn chat(&self, message: &str) -> Result<String>;
}

/// Output from agent execution.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub response: String,
    pub tool_calls: Vec<ToolCall>,
    pub success: bool,
}

/// Record of a tool call made by the agent.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
}
