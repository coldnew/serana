//! Agent module - Hermes-style autonomous coding agent.
//!
//! Core components:
//! - `Agent` trait - the agent interface
//! - `IterationBudget` - turn counting and limits
//! - `AgentCallbacks` - progress notifications
//! - `CancelToken` - interruptible operations
//! - `PromptBuilder` - system prompt assembly

use async_trait::async_trait;
use serde_json::Value;
use crate::Result;

pub mod coding;
pub mod iteration_budget;
pub mod callbacks;
pub mod interruptible;
pub mod message_validation;
pub mod prompt_builder;
pub mod tool_executor;
pub mod session;
pub mod compressor;
pub mod tool_approval;
pub mod subagent;
pub mod meta_cognition;

pub use iteration_budget::{IterationBudget, DEFAULT_MAX_ITERATIONS, DEFAULT_SUBAGENT_MAX_ITERATIONS};
pub use callbacks::{AgentCallbacks, AgentStatus, CallbackState};
pub use interruptible::{CancelToken, InterruptibleApiCall};
pub use message_validation::{validate_message_alternation, fix_message_alternation};
pub use prompt_builder::PromptBuilder;

pub use tool_executor::{execute_tools_concurrent, ToolExecutionResult};
pub use session::{Session, SessionMeta, SessionStore, SearchResult, StoredMessage, StoredToolCall};
pub use compressor::{CompressionConfig, CompressionDecision, CompressionThresholds, ContextCompressor};
pub use tool_approval::{ApprovalDecision, ApprovalMode, RiskLevel, ToolApproval};
pub use subagent::{delegate_task, SubagentConfig, SubagentResult, SubagentSpawner, SubagentTask};
pub use meta_cognition::{MetaCognition, ModificationKind, ModificationRecord};

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
