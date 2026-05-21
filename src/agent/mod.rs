//! Agent module - Hermes-style autonomous coding agent.
//!
//! Core components:
//! - `Agent` trait - the agent interface
//! - `IterationBudget` - turn counting and limits
//! - `AgentCallbacks` - progress notifications
//! - `CancelToken` - interruptible operations
//! - `PromptBuilder` - system prompt assembly

use crate::Result;
use async_trait::async_trait;
use serde_json::Value;

pub mod callbacks;
pub mod coding;
pub mod compressor;
pub mod interruptible;
pub mod iteration_budget;
pub mod message_validation;
pub mod meta_cognition;
pub mod prompt_builder;
pub mod session;
pub mod subagent;
pub mod tool_approval;
pub mod tool_executor;
pub use callbacks::{AgentCallbacks, AgentStatus, CallbackState};
pub use interruptible::{CancelToken, InterruptibleApiCall};
pub use iteration_budget::{
    IterationBudget, DEFAULT_MAX_ITERATIONS, DEFAULT_SUBAGENT_MAX_ITERATIONS,
};
pub use message_validation::{fix_message_alternation, validate_message_alternation};
pub use meta_cognition::{
    MetaCognition, MetaRecord, ModificationKind, ModificationRecord, ModificationStats,
};
pub use prompt_builder::PromptBuilder;

pub use compressor::{
    CompressionConfig, CompressionDecision, CompressionThresholds, ContextCompressor,
};
pub use session::{
    SearchResult, Session, SessionMeta, SessionStore, StoredMessage, StoredToolCall,
};
pub use subagent::{delegate_task, SubagentConfig, SubagentResult, SubagentSpawner, SubagentTask};
pub use tool_approval::{ApprovalDecision, ApprovalMode, RiskLevel, ToolApproval};
pub use tool_executor::{execute_tools_concurrent, ToolExecutionResult};

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
