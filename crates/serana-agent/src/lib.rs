#[deprecated(
    note = "Use the hermes module and HermesAgent; coding is only a compatibility module."
)]
pub mod coding;
pub mod checkpoint;
pub mod compactor;
pub mod compression_gate;
pub mod compressor;
pub mod engine;
pub mod factory;
pub mod gatherer;
pub mod hermes;
pub mod lifecycle;
pub mod message_validation;
pub mod prompt_builder;
pub mod run_state;
pub mod runtime_config;
pub mod session;
pub mod session_recorder;
pub mod stream_rules;
pub mod subagent;
pub mod tool_call_validator;
pub mod tool_executor;
pub mod tool_turn;
pub mod turn_runner;

#[allow(deprecated)]
pub use coding::CodingAgent;
pub use checkpoint::CheckpointManager;
pub use compactor::ContextCompactor;
pub use compression_gate::{CompressionGate, CompressionGateOutcome};
pub use compressor::ContextCompressor;
pub use engine::{AgentEngine, AgentEngineParts};
pub use factory::AgentFactory;
pub use gatherer::ContextGatherer;
pub use hermes::HermesAgent;
pub use lifecycle::AgentLifecycle;
pub use message_validation::{fix_message_alternation, validate_message_alternation};
pub use prompt_builder::PromptBuilder;
pub use run_state::AgentRunState;
pub use runtime_config::{AgentPromptConfig, AgentRuntimeConfig};
pub use session::{
    SearchResult, Session, SessionMeta, SessionStore, StoredMessage, StoredToolCall,
};
pub use session_recorder::SessionRecorder;
pub use stream_rules::{
    ContextMode, InterruptMode, RepeatPolicy, RuleScope, StreamRule, StreamRuleEngine,
    StreamRuleMatch,
};
pub use subagent::{delegate_task, SubagentConfig, SubagentResult, SubagentSpawner, SubagentTask};
pub use tool_call_validator::ToolCallValidator;
pub use tool_executor::{execute_tools_concurrent, ToolExecutionResult};
pub use tool_turn::{handle_tool_turn, ToolTurnOutput};
pub use turn_runner::{TurnOutcome, TurnRunner};

#[cfg(test)]
mod test_support {
    use std::path::{Path, PathBuf};

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    pub fn tempdir() -> std::io::Result<TempDir> {
        let path = std::env::temp_dir().join(format!("serana-agent-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path)?;
        Ok(TempDir { path })
    }
}
