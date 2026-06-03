pub mod agent;
pub mod callbacks;
pub mod compression;
pub mod config;
pub mod context;
pub mod interruptible;
pub mod iteration_budget;
pub mod llm_client;
pub mod message;
pub mod meta_cognition;
pub mod token_counter;
pub mod tool;
pub mod tool_approval;
pub mod verification;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

pub use agent::{Agent, AgentOutput, ToolCall};
pub use callbacks::{AgentCallbacks, AgentStatus, CallbackState};
pub use compression::{
    CompactionAction, CompactionMethod, CompactionResult, CompactionStats, CompressionConfig,
    CompressionDecision, CompressionThresholds, ToolTokenDetail,
};
pub use config::{Config, LegacyProviderConfig, LlmConfig, ProviderConfig};
pub use context::Context;
pub use interruptible::{CancelToken, InterruptibleApiCall};
pub use iteration_budget::{
    IterationBudget, TokenCost, DEFAULT_MAX_ITERATIONS, DEFAULT_SUBAGENT_MAX_ITERATIONS,
};
pub use llm_client::{FunctionDefinition, LlmClient, ToolDefinition};
pub use message::{FunctionCall, Message, ToolCallData};
pub use meta_cognition::{
    MetaCognition, MetaRecord, ModificationKind, ModificationRecord, ModificationStats,
};
pub use token_counter::TokenCounter;
pub use tool::Tool;
pub use tool_approval::{ApprovalDecision, ApprovalMode, RiskLevel, ToolApproval};
pub use verification::{StateSnapshot, VerificationResult, VerificationSystem};
