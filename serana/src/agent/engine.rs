use std::sync::Arc;

use crate::core::{
    AgentCallbacks, AgentOutput, AgentStatus, CancelToken, IterationBudget, LlmClient, Message,
    MetaCognition, Result,
};
use crate::llm::AuxiliaryClient;
use crate::tools::ToolRegistry;

use super::stream_rules::ContextMode;
use super::{
    handle_tool_turn, validate_message_alternation, AgentLifecycle, AgentRunState,
    CheckpointManager, CompressionGate, CompressionGateOutcome, ContextCompressor, PromptBuilder,
    SessionRecorder, StreamRuleEngine, ToolCallValidator, TurnRunner,
};

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

            self.parts.callbacks.fire_status(AgentStatus::Thinking);
            let messages_snapshot = state.messages().to_vec();
            let tools_snapshot = tools.clone();
            let runner = TurnRunner::new(self.parts.llm, self.parts.callbacks);

            // Run with TTSR if available
            let response = if let Some(ref mut rules_engine) = self.parts.stream_rules {
                use super::turn_runner::TurnOutcome;
                match runner
                    .run_with_ttsr(&messages_snapshot, &tools_snapshot, Some(rules_engine))
                    .await?
                {
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
                                // Inject reminder and retry the turn
                                state.push_system_message(&injection);
                                self.parts
                                    .callbacks
                                    .fire_stream_delta(&format!("\n[TTSR: {} — retrying]\n", name));
                                continue;
                            }
                            ContextMode::Keep => {
                                // Keep partial output, queue injection for next turn
                                deferred_injections.push(injection);
                                self.parts
                                    .callbacks
                                    .fire_stream_delta(&format!("\n[TTSR: {} — deferred]\n", name));
                                // Treat as a text response with partial content
                                Message::assistant(partial_content)
                            }
                        }
                    }
                }
            } else {
                runner.run(&messages_snapshot, &tools_snapshot).await?
            };

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
