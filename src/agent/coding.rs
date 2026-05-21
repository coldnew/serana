use async_trait::async_trait;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use crate::agent::{
    Agent, AgentOutput, AgentCallbacks, AgentStatus,
    IterationBudget, PromptBuilder, execute_tools_concurrent, validate_message_alternation,
    SessionStore, ContextCompressor, CompressionDecision,
};
use crate::llm::{LlmClient, Message, ToolDefinition, FunctionDefinition, AuxiliaryClient};
use crate::tools::ToolRegistry;
use crate::Result;

/// Hermes-style coding agent with iteration budget and callback support.
pub struct CodingAgent {
    llm: Box<dyn LlmClient>,
    auxiliary: Option<Arc<AuxiliaryClient>>,
    tools: ToolRegistry,
    budget: IterationBudget,
    callbacks: AgentCallbacks,
    prompt_builder: PromptBuilder,
    session_store: Option<SessionStore>,
    session_id: Option<String>,
    compressor: ContextCompressor,
}

impl CodingAgent {
    pub fn new(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> Self {
        Self {
            llm,
            auxiliary: None,
            tools,
            budget: IterationBudget::default_parent(),
            callbacks: AgentCallbacks::new(),
            prompt_builder: PromptBuilder::new(PathBuf::from(".")),
            session_store: None,
            session_id: None,
            compressor: ContextCompressor::with_defaults(),
        }
    }

    /// Set callbacks for progress notifications.
    pub fn with_callbacks(mut self, callbacks: AgentCallbacks) -> Self {
        self.callbacks = callbacks;
        self
    }

    /// Set custom iteration budget.
    pub fn with_budget(mut self, budget: IterationBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Set workspace path for prompt builder.
    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.prompt_builder = PromptBuilder::new(workspace);
        self
    }

    /// Enable session persistence.
    pub fn with_session(mut self, store: SessionStore, session_id: String) -> Self {
        self.session_store = Some(store);
        self.session_id = Some(session_id);
        self
    }

    /// Set custom context compressor.
    pub fn with_compressor(mut self, compressor: ContextCompressor) -> Self {
        self.compressor = compressor;
        self
    }

    /// Set auxiliary client for background tasks (compression, validation, etc).
    pub fn with_auxiliary(mut self, auxiliary: AuxiliaryClient) -> Self {
        self.auxiliary = Some(Arc::new(auxiliary));
        self
    }
}

#[async_trait]
impl Agent for CodingAgent {
    fn name(&self) -> &'static str {
        "coding-agent"
    }

    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        // Notify status
        self.callbacks.fire_status(AgentStatus::Running);

        let system_prompt = self.prompt_builder.build();
        let tools = self.build_tool_definitions();
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(instruction.to_string()),
        ];
        self.persist_message("user", instruction)?;

        // Generate session title in background if auxiliary available
        if let (Some(store), Some(session_id), Some(aux)) = 
            (&self.session_store, &self.session_id, &self.auxiliary) {
            let store = store.clone();
            let sid = session_id.clone();
            let aux = aux.clone();
            let first_msg = instruction.chars().take(200).collect::<String>();
            tokio::spawn(async move {
                if let Ok(title) = aux.generate_title(&first_msg).await {
                    let _ = store.update_session_title(&sid, &title);
                }
            });
        }

        let mut all_tool_calls = Vec::new();

        // Use iteration budget instead of hardcoded limit
        while self.budget.remaining() > 0 {
            match self.compressor.check_compression(self.compressor.estimate_tokens(&messages)) {
                CompressionDecision::Gateway => {
                    self.callbacks.fire_status(AgentStatus::Compressing);
                    messages = self.compress_messages(&messages).await?;
                    self.callbacks.fire_status(AgentStatus::Running);
                }
                CompressionDecision::Preflight | CompressionDecision::None => {}
            }

            // Validate message alternation before each LLM call
            if let Err(e) = validate_message_alternation(&messages) {
                anyhow::bail!("Message alternation error: {}", e);
            }

            // Validate risky tool calls if auxiliary available
            self.validate_tool_calls_before_execution(&messages, &tools).await?;

            // Use streaming API for real-time response
            // Clone messages and tools so stream can own them without blocking mutations
            self.callbacks.fire_status(AgentStatus::Thinking);
            let messages_snapshot = messages.clone();
            let tools_snapshot = tools.clone();
            let response = self.stream_llm_call(&messages_snapshot, &tools_snapshot).await?;
            self.callbacks.fire_status(AgentStatus::Running);

            match response {
                Message::ToolCall { role, content, tool_calls } => {
                    if let Some(content) = content.as_deref() {
                        self.persist_message("assistant", content)?;
                    }
                    // Add assistant message with tool calls to history
                    messages.push(Message::ToolCall { role, content, tool_calls: tool_calls.clone() });

                    // Execute tools
                    self.callbacks.fire_status(AgentStatus::ExecutingTool);
                    let results = execute_tools_concurrent(&tool_calls, &self.tools, &self.callbacks).await;
                    self.callbacks.fire_status(AgentStatus::Running);

                    // Process results
                    for result in results {
                        let tool_call = result.to_tool_call();
                        self.persist_tool_call(&tool_call)?;
                        let result_str = result.result_string();
                        messages.push(Message::tool_result(result.id, result_str));
                        all_tool_calls.push(tool_call);
                    }

                    // Check if budget exceeded after iteration
                    if self.budget.increment() {
                        self.callbacks.fire_status(AgentStatus::BudgetExhausted);
                        break;
                    }
                }
                Message::Text { role: _, content } => {
                    // Final response - no more tool calls
                    self.persist_message("assistant", &content)?;
                    self.callbacks.fire_status(AgentStatus::Complete);
                    return Ok(AgentOutput {
                        response: content,
                        tool_calls: all_tool_calls,
                        success: true,
                    });
                }
                Message::ToolResult { .. } => {
                    anyhow::bail!("Unexpected tool result message from LLM");
                }
            }
        }

        self.callbacks.fire_status(AgentStatus::BudgetExhausted);

        anyhow::bail!("Exceeded iteration budget")
    }

    async fn chat(&self, message: &str) -> Result<String> {
        let messages = vec![Message::user(message.to_string())];
        self.llm.chat(&messages).await
    }
}

impl CodingAgent {
    fn build_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.list().iter().filter_map(|name| {
            self.tools.get(name).map(|tool| {
                ToolDefinition {
                    r#type: "function".to_string(),
                    function: FunctionDefinition {
                        name: tool.name().to_string(),
                        description: tool.description().to_string(),
                        parameters: serde_json::json!({"type": "object", "properties": {}}),
                    },
                }
            })
        }).collect()
    }

    fn persist_message(&self, role: &str, content: &str) -> Result<()> {
        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            store.save_message(session_id, role, content)?;
        }
        Ok(())
    }

    fn persist_tool_call(&self, tool_call: &crate::agent::ToolCall) -> Result<()> {
        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            store.save_tool_call(
                session_id,
                &tool_call.name,
                &tool_call.arguments,
                tool_call.result.as_ref(),
            )?;
        }
        Ok(())
    }

    /// Compress messages using auxiliary client if available, otherwise main LLM.
    async fn compress_messages(&self, messages: &[Message]) -> Result<Vec<Message>> {
        if let Some(aux) = &self.auxiliary {
            self.compressor.compress_messages_with_auxiliary(messages, aux.as_ref()).await
        } else {
            self.compressor.compress_messages(messages, self.llm.as_ref()).await
        }
    }

    /// Validate tool calls before execution using auxiliary client.
    async fn validate_tool_calls_before_execution(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<()> {
        // Tool validation is handled in tool_executor.rs via callbacks
        // This is a placeholder for pre-execution validation if needed
        Ok(())
    }

    /// Call LLM with streaming, consuming the stream and firing callbacks.
    async fn stream_llm_call(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let mut stream = self.llm.chat_with_tools_stream(messages, tools);
        let mut final_message: Option<Message> = None;
        let mut accumulated_content = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(Message::Text { content, .. }) => {
                    // Stream delta - fire callback for each chunk
                    self.callbacks.fire_stream_delta(&content);
                    accumulated_content.push_str(&content);
                }
                Ok(msg) => {
                    // Non-streaming message (tool call or complete)
                    final_message = Some(msg);
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // If we accumulated content but no final message, construct one
        if final_message.is_none() && !accumulated_content.is_empty() {
            final_message = Some(Message::assistant(accumulated_content));
        }

        final_message.ok_or_else(|| anyhow::anyhow!("No message received from stream"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::tempdir;

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

        let agent = CodingAgent::new(Box::new(MockLlm), ToolRegistry::new())
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
            self.saw_summary.store(saw_summary, std::sync::atomic::Ordering::SeqCst);
            Ok(Message::assistant("done".to_string()))
        }
    }

    #[tokio::test]
    async fn compresses_context_before_llm_gateway_call() {
        let saw_summary = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let compressor = ContextCompressor::new(crate::agent::CompressionConfig {
            max_tokens: 1,
            protect_last_n: 0,
            thresholds: crate::agent::CompressionThresholds {
                preflight: 0.0,
                gateway: 0.0,
            },
        });
        let agent = CodingAgent::new(
            Box::new(CompressionAwareLlm { saw_summary: saw_summary.clone() }),
            ToolRegistry::new(),
        )
        .with_compressor(compressor);

        let output = agent.execute("compress this before model call").await.unwrap();
        assert_eq!(output.response, "done");
        assert!(saw_summary.load(std::sync::atomic::Ordering::SeqCst));
    }
}
