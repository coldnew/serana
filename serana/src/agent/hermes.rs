use super::{
    AgentEngine, AgentEngineParts, AgentPromptConfig, AgentRuntimeConfig, CheckpointManager,
    ContextCompressor, PromptBuilder, SessionRecorder, SessionStore, StreamRuleEngine,
};
use crate::core::{
    Agent, AgentCallbacks, AgentOutput, CancelToken, IterationBudget, LlmClient, Message,
    MetaCognition, Result,
};
use crate::llm::AuxiliaryClient;
use crate::tools::ToolRegistry;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct HermesAgent {
    llm: Box<dyn LlmClient>,
    auxiliary: Option<Arc<AuxiliaryClient>>,
    tools: ToolRegistry,
    budget: IterationBudget,
    callbacks: AgentCallbacks,
    prompt_builder: PromptBuilder,
    session_recorder: SessionRecorder,
    compressor: ContextCompressor,
    cancel_token: Option<CancelToken>,
    #[allow(dead_code)]
    meta_cognition: Arc<MetaCognition>,
    checkpoint_manager: CheckpointManager,
    stream_rules: Mutex<Option<StreamRuleEngine>>,
}

impl HermesAgent {
    #[deprecated(
        note = "Use HermesAgent::with_tools for a custom registry or HermesAgent::hermes for the powered Hermes toolset."
    )]
    pub fn new(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> Self {
        Self::with_tools(llm, tools)
    }

    pub fn hermes(llm: Box<dyn LlmClient>) -> Self {
        Self::with_tools(llm, ToolRegistry::hermes())
    }

    #[deprecated(note = "Use HermesAgent::hermes for the canonical powered Hermes constructor.")]
    pub fn powered(llm: Box<dyn LlmClient>) -> Self {
        Self::hermes(llm)
    }

    pub fn with_tools(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> Self {
        Self {
            llm,
            auxiliary: None,
            tools,
            budget: IterationBudget::default_parent(),
            callbacks: AgentCallbacks::new(),
            prompt_builder: PromptBuilder::new(PathBuf::from(".")),
            session_recorder: SessionRecorder::disabled(),
            compressor: ContextCompressor::with_defaults(),
            cancel_token: None,
            meta_cognition: Arc::new(MetaCognition::new()),
            checkpoint_manager: CheckpointManager::new(),
            stream_rules: Mutex::new(None),
        }
    }

    pub fn with_callbacks(mut self, callbacks: AgentCallbacks) -> Self {
        self.callbacks = callbacks;
        self
    }

    pub fn with_budget(mut self, budget: IterationBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn with_runtime_config(mut self, config: AgentRuntimeConfig) -> Self {
        self.apply_prompt_config(config.prompt_config());
        self
    }

    pub fn with_prompt_config(mut self, config: AgentPromptConfig) -> Self {
        self.apply_prompt_config(config);
        self
    }

    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.prompt_builder = PromptBuilder::new(workspace);
        self
    }

    pub fn with_session(mut self, store: SessionStore, session_id: String) -> Self {
        self.session_recorder = SessionRecorder::new(store, session_id);
        self
    }

    pub fn with_compressor(mut self, compressor: ContextCompressor) -> Self {
        self.compressor = compressor;
        self
    }

    pub fn with_cancel_token(mut self, token: CancelToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.prompt_builder = self.prompt_builder.with_skills(skills);
        self
    }
    pub fn with_auxiliary(mut self, auxiliary: AuxiliaryClient) -> Self {
        self.auxiliary = Some(Arc::new(auxiliary));
        self
    }

    pub fn with_stream_rules(mut self, rules: StreamRuleEngine) -> Self {
        self.stream_rules = Mutex::new(Some(rules));
        self
    }

    fn apply_prompt_config(&mut self, config: AgentPromptConfig) {
        self.prompt_builder = PromptBuilder::new(config.workspace).with_skills(config.skills);
        self.compressor = config.compressor;
    }
}

#[async_trait]
impl Agent for HermesAgent {
    fn name(&self) -> &'static str {
        "hermes-agent"
    }

    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        // Take stream_rules out of Mutex for the duration of execution
        let mut rules_guard = self.stream_rules.lock().await;
        let mut rules_opt = rules_guard.take();

        let result = AgentEngine::new(AgentEngineParts {
            llm: self.llm.as_ref(),
            auxiliary: self.auxiliary.clone(),
            tools: &self.tools,
            budget: &self.budget,
            callbacks: &self.callbacks,
            prompt_builder: &self.prompt_builder,
            session_recorder: &self.session_recorder,
            compressor: &self.compressor,
            cancel_token: self.cancel_token.as_ref(),
            meta_cognition: &self.meta_cognition,
            checkpoint_manager: &self.checkpoint_manager,
            stream_rules: rules_opt.as_mut(),
        })
        .execute(instruction)
        .await;

        // Put stream_rules back
        *rules_guard = rules_opt;

        result
    }

    async fn chat(&self, message: &str) -> Result<String> {
        let messages = vec![Message::user(message.to_string())];
        self.llm.chat(&messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::test_support::tempdir;
    use crate::core::{CompressionConfig, CompressionThresholds, ToolDefinition};

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("compressed".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("hello from agent".to_string()))
        }
    }

    #[tokio::test]
    async fn persists_user_and_assistant_messages_when_session_enabled() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions.db"));
        store.init().unwrap();
        let session = store.create_session().unwrap();

        let agent = HermesAgent::hermes(Box::new(MockLlm))
            .with_session(store.clone(), session.meta.id.clone());

        let output = agent.execute("say hello").await.unwrap();
        assert_eq!(output.response, "hello from agent");

        let loaded = store.load_session(&session.meta.id).unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].role, "user");
        assert_eq!(loaded.messages[0].content, "say hello");
        assert_eq!(loaded.messages[1].role, "assistant");
        assert_eq!(loaded.messages[1].content, "hello from agent");
    }

    struct CompressionAwareLlm {
        saw_summary: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl LlmClient for CompressionAwareLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("important prior context".to_string())
        }

        async fn chat_with_tools(
            &self,
            messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            let saw_summary = messages.iter().any(|message| match message {
                Message::Text { content, .. } => content.contains("Previous conversation summary"),
                _ => false,
            });
            self.saw_summary
                .store(saw_summary, std::sync::atomic::Ordering::SeqCst);
            Ok(Message::assistant("done".to_string()))
        }
    }

    #[tokio::test]
    async fn compresses_context_before_llm_gateway_call() {
        let saw_summary = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let compressor = ContextCompressor::new(CompressionConfig {
            max_tokens: 1,
            protect_last_n: 0,
            thresholds: CompressionThresholds {
                preflight: 0.0,
                gateway: 0.0,
            },
        });
        let agent = HermesAgent::with_tools(
            Box::new(CompressionAwareLlm {
                saw_summary: saw_summary.clone(),
            }),
            ToolRegistry::hermes(),
        )
        .with_compressor(compressor);

        let output = agent
            .execute("compress this before model call")
            .await
            .unwrap();
        assert_eq!(output.response, "done");
        assert!(saw_summary.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn reports_hermes_agent_name() {
        let agent = HermesAgent::hermes(Box::new(MockLlm));

        assert_eq!(agent.name(), "hermes-agent");
    }

    #[test]
    fn hermes_constructor_uses_hermes_tools() {
        let agent = HermesAgent::hermes(Box::new(MockLlm));

        assert!(agent.tools.get("read_self").is_some());
        assert!(agent.tools.get("web_search").is_some());
    }

    #[test]
    #[allow(deprecated)]
    fn powered_constructor_remains_hermes_compatibility() {
        let agent = HermesAgent::powered(Box::new(MockLlm));

        assert!(agent.tools.get("read_self").is_some());
        assert!(agent.tools.get("web_search").is_some());
    }

    #[test]
    fn runtime_config_applies_prompt_fields_only() {
        let agent = HermesAgent::with_tools(Box::new(MockLlm), ToolRegistry::core())
            .with_runtime_config(
                AgentRuntimeConfig::default().with_tool_profile(crate::tools::ToolProfile::Hermes),
            );

        assert!(agent.tools.get("read_self").is_none());
        assert!(agent.tools.get("read_file").is_some());
    }
}
