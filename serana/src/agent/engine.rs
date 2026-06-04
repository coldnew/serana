use std::sync::Arc;
use std::time::Duration;

use crate::core::{
    AgentCallbacks, AgentOutput, AgentStatus, CancelToken, IterationBudget, LlmClient, Message,
    MetaCognition, Result, RetryConfig,
};
use crate::llm::AuxiliaryClient;
use crate::tools::ToolRegistry;

use super::stream_rules::ContextMode;
use super::{
    handle_tool_turn, validate_message_alternation, AgentLifecycle, AgentRunState,
    CheckpointManager, CompressionGate, CompressionGateOutcome, ContextCompressor, PromptBuilder,
    SessionRecorder, StreamRuleEngine, ToolCallValidator, TurnRunner,
};

/// Check if an error is a transient LLM failure worth retrying.
fn is_retryable_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    // Rate limits and overloaded
    if msg.contains("rate limit")
        || msg.contains("ratelimit")
        || msg.contains("429")
        || msg.contains("too many requests")
        || msg.contains("overloaded")
        || msg.contains("capacity")
    {
        return true;
    }
    // Server errors
    if msg.contains("500")
        || msg.contains("502")
        || msg.contains("503")
        || msg.contains("529")
        || msg.contains("internal server error")
        || msg.contains("bad gateway")
        || msg.contains("service unavailable")
    {
        return true;
    }
    // Network errors
    if msg.contains("connection reset")
        || msg.contains("connection refused")
        || msg.contains("timed out")
        || msg.contains("timeout")
        || msg.contains("eof")
        || msg.contains("broken pipe")
    {
        return true;
    }
    false
}

/// Check if an error is a context overflow (too many tokens).
fn is_context_overflow(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("context_length_exceeded")
        || msg.contains("context window")
        || msg.contains("context length")
        || msg.contains("too many tokens")
        || msg.contains("maximum context")
        || msg.contains("max_tokens")
        || msg.contains("prompt is too long")
        || (msg.contains("invalid_request_error") && msg.contains("context"))
}

pub struct AgentEngineParts<'a> {
    pub llm: &'a dyn LlmClient,
    pub auxiliary: Option<Arc<AuxiliaryClient>>,
    pub tools: &'a ToolRegistry,
    pub budget: &'a IterationBudget,
    pub callbacks: &'a AgentCallbacks,
    pub prompt_builder: &'a PromptBuilder,
    pub session_recorder: &'a SessionRecorder,
    pub compressor: &'a ContextCompressor,
    pub cancel_token: Option<&'a CancelToken>,
    pub meta_cognition: &'a Arc<MetaCognition>,
    pub checkpoint_manager: &'a CheckpointManager,
    pub stream_rules: Option<&'a mut StreamRuleEngine>,
    pub retry_config: RetryConfig,
}

pub struct AgentEngine<'a> {
    parts: AgentEngineParts<'a>,
}

impl<'a> AgentEngine<'a> {
    pub fn new(parts: AgentEngineParts<'a>) -> Self {
        Self { parts }
    }

    pub async fn execute(&mut self, instruction: &str) -> Result<AgentOutput> {
        self.parts.callbacks.fire_status(AgentStatus::Running);

        let system_prompt = self.parts.prompt_builder.build();
        let tools = self.parts.tools.definitions();
        let mut state = AgentRunState::new(system_prompt, instruction.to_string());
        self.parts
            .session_recorder
            .save_message("user", instruction)?;
        self.parts
            .session_recorder
            .generate_title_async(self.parts.auxiliary.clone(), instruction);

        let lifecycle = AgentLifecycle::new(
            self.parts.budget,
            self.parts.cancel_token,
            self.parts.callbacks,
        );

        // Collect deferred TTSR injections to append after tool turns
        let mut deferred_injections: Vec<String> = Vec::new();
        let mut overflow_recovery_attempted = false;

        while lifecycle.can_continue() {
            lifecycle.check_cancelled()?;

            match CompressionGate::new(
                self.parts.compressor,
                self.parts.llm,
                self.parts.auxiliary.clone(),
            )
            .check(state.messages())
            .await?
            {
                CompressionGateOutcome::Compressed(messages) => {
                    self.parts.callbacks.fire_status(AgentStatus::Compressing);
                    state.replace_messages(messages);
                    self.parts.callbacks.fire_status(AgentStatus::Running);
                }
                CompressionGateOutcome::Unchanged => {}
            }

            if let Err(e) = validate_message_alternation(state.messages()) {
                anyhow::bail!("Message alternation error: {}", e);
            }

            // --- LLM turn with auto-retry ---
            let mut retry_attempt = 0u32;
            let response = loop {
                self.parts.callbacks.fire_status(AgentStatus::Thinking);
                let messages_snapshot = state.messages().to_vec();
                let tools_snapshot = tools.clone();
                let runner = TurnRunner::new(self.parts.llm, self.parts.callbacks);

                // Run with TTSR if available
                let turn_result = if let Some(ref mut rules_engine) = self.parts.stream_rules {
                    use super::turn_runner::TurnOutcome;
                    match runner
                        .run_with_ttsr(&messages_snapshot, &tools_snapshot, Some(rules_engine))
                        .await
                    {
                        Ok(outcome) => Ok(match outcome {
                            TurnOutcome::Complete(msg) => msg,
                            TurnOutcome::Interrupted {
                                name,
                                injection,
                                context,
                                partial_content,
                            } => {
                                // Mark rule as triggered
                                self.parts
                                    .stream_rules
                                    .as_mut()
                                    .unwrap()
                                    .mark_triggered(&name);

                                match context {
                                    ContextMode::Discard => {
                                        state.push_system_message(&injection);
                                        self.parts.callbacks.fire_stream_delta(&format!(
                                            "\n[TTSR: {} — retrying]\n",
                                            name
                                        ));
                                        continue;
                                    }
                                    ContextMode::Keep => {
                                        deferred_injections.push(injection);
                                        self.parts.callbacks.fire_stream_delta(&format!(
                                            "\n[TTSR: {} — deferred]\n",
                                            name
                                        ));
                                        Message::assistant(partial_content)
                                    }
                                }
                            }
                        }),
                        Err(e) => Err(e),
                    }
                } else {
                    runner
                        .run(&messages_snapshot, &tools_snapshot)
                        .await
                        .map(|msg| msg)
                };

                match turn_result {
                    Ok(msg) => break msg,
                    Err(e) => {
                        // Context overflow → try compaction once, then retry
                        if is_context_overflow(&e) && !overflow_recovery_attempted {
                            overflow_recovery_attempted = true;
                            self.parts.callbacks.fire_status(AgentStatus::Compressing);
                            self.parts.callbacks.fire_stream_delta(
                                "\n[Context overflow detected — compacting...]\n",
                            );
                            match CompressionGate::new(
                                self.parts.compressor,
                                self.parts.llm,
                                self.parts.auxiliary.clone(),
                            )
                            .check(state.messages())
                            .await?
                            {
                                CompressionGateOutcome::Compressed(messages) => {
                                    state.replace_messages(messages);
                                    self.parts.callbacks.fire_status(AgentStatus::Running);
                                    retry_attempt = 0;
                                    continue;
                                }
                                CompressionGateOutcome::Unchanged => {
                                    // Compaction didn't help, propagate error
                                    self.parts.callbacks.fire_status(AgentStatus::Running);
                                    return Err(e);
                                }
                            }
                        }

                        // Transient error → retry with backoff
                        if self.parts.retry_config.enabled
                            && is_retryable_error(&e)
                            && retry_attempt < self.parts.retry_config.max_retries
                        {
                            retry_attempt += 1;
                            let delay_ms = (self.parts.retry_config.base_delay_ms
                                * 2u64.pow(retry_attempt - 1))
                            .min(self.parts.retry_config.max_delay_ms);
                            self.parts.callbacks.fire_stream_delta(&format!(
                                "\n[Retry {}/{} after {}ms — {}]\n",
                                retry_attempt, self.parts.retry_config.max_retries, delay_ms, e
                            ));
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }

                        return Err(e);
                    }
                }
            };
            // --- End LLM turn with auto-retry ---

            self.parts.callbacks.fire_status(AgentStatus::Running);

            match response {
                Message::ToolCall {
                    role,
                    content,
                    tool_calls,
                } => {
                    ToolCallValidator::new(&tools).validate(&tool_calls)?;
                    if let Some(content) = content.as_deref() {
                        self.parts
                            .session_recorder
                            .save_message("assistant", content)?;
                    }
                    state.push_assistant_tool_call(role, content, tool_calls.clone());

                    let turn = handle_tool_turn(
                        &tool_calls,
                        self.parts.tools,
                        self.parts.callbacks,
                        self.parts.meta_cognition,
                        None,
                    )
                    .await;

                    // Process checkpoint/rewind signals from tool results
                    for (_i, msg) in turn.messages.iter().enumerate() {
                        if let Message::ToolResult { content, .. } = msg {
                            if let Ok(result_json) =
                                serde_json::from_str::<serde_json::Value>(content)
                            {
                                if let Some(label) =
                                    CheckpointManager::is_checkpoint_signal(&result_json)
                                {
                                    let idx = state.messages().len();
                                    self.parts.checkpoint_manager.save(label, idx);
                                }
                                if let Some(label_opt) =
                                    CheckpointManager::is_rewind_signal(&result_json)
                                {
                                    let target =
                                        self.parts.checkpoint_manager.find_rewind_target(label_opt);
                                    if let Some(target_idx) = target {
                                        state.truncate_to(target_idx);
                                        self.parts.checkpoint_manager.clear_after(target_idx);
                                        self.parts.callbacks.fire_stream_delta(&format!(
                                            "\n[Rewound to checkpoint at message {}]\n",
                                            target_idx
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    for tool_call in &turn.tool_calls {
                        self.parts.session_recorder.save_tool_call(tool_call)?;
                    }
                    state.apply_tool_turn(turn);

                    // Inject any deferred TTSR injections as system messages
                    for injection in deferred_injections.drain(..) {
                        state.push_system_message(&injection);
                    }

                    // Advance TTSR turn counter
                    if let Some(ref mut rules_engine) = self.parts.stream_rules {
                        rules_engine.advance_turn();
                    }

                    // Reset overflow recovery flag after a successful tool turn
                    overflow_recovery_attempted = false;

                    if lifecycle.complete_tool_iteration() {
                        break;
                    }
                }
                Message::Text { role: _, content } => {
                    self.parts
                        .session_recorder
                        .save_message("assistant", &content)?;

                    // Inject any deferred TTSR injections as system messages
                    for injection in deferred_injections.drain(..) {
                        state.push_system_message(&injection);
                    }

                    // Advance TTSR turn counter
                    if let Some(ref mut rules_engine) = self.parts.stream_rules {
                        rules_engine.advance_turn();
                    }

                    self.parts.callbacks.fire_status(AgentStatus::Complete);
                    return Ok(state.output(content));
                }
                Message::ToolResult { .. } => {
                    anyhow::bail!("Unexpected tool result message from LLM");
                }
            }
        }

        lifecycle.fire_budget_exhausted();

        anyhow::bail!("Exceeded iteration budget")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_error_detection() {
        // Rate limit
        assert!(is_retryable_error(&anyhow::anyhow!(
            "Error 429: Too Many Requests"
        )));
        assert!(is_retryable_error(&anyhow::anyhow!("rate limit exceeded")));
        assert!(is_retryable_error(&anyhow::anyhow!("overloaded")));

        // Server errors
        assert!(is_retryable_error(&anyhow::anyhow!(
            "503 Service Unavailable"
        )));
        assert!(is_retryable_error(&anyhow::anyhow!("502 Bad Gateway")));

        // Network errors
        assert!(is_retryable_error(&anyhow::anyhow!(
            "connection reset by peer"
        )));
        assert!(is_retryable_error(&anyhow::anyhow!("request timed out")));

        // Non-retryable
        assert!(!is_retryable_error(&anyhow::anyhow!("invalid API key")));
        assert!(!is_retryable_error(&anyhow::anyhow!("model not found")));
    }

    #[test]
    fn context_overflow_detection() {
        assert!(is_context_overflow(&anyhow::anyhow!(
            "context_length_exceeded: maximum context length is 128000 tokens"
        )));
        assert!(is_context_overflow(&anyhow::anyhow!(
            "This model's maximum context length is 128000"
        )));
        assert!(is_context_overflow(&anyhow::anyhow!(
            "prompt is too long: 200000 tokens"
        )));
        assert!(!is_context_overflow(&anyhow::anyhow!(
            "rate limit exceeded"
        )));
    }
}
