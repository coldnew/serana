use async_trait::async_trait;
use serde_json::Value;
use crate::Result;

pub mod coding;

#[async_trait]
pub trait Agent: Send + Sync {
    /// Name of the agent
    fn name(&self) -> &'static str;

    /// Execute a task given natural language instruction
    async fn execute(&self, instruction: &str) -> Result<AgentOutput>;

    /// Process a message and return response (for interactive mode)
    async fn chat(&self, message: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub response: String,
    pub tool_calls: Vec<ToolCall>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
}
