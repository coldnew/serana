use std::sync::Arc;

use serana_core::{
    AgentCallbacks, AgentOutput, AgentStatus, CancelToken, IterationBudget, LlmClient, Message,
    MetaCognition, Result,
};
use serana_llm::AuxiliaryClient;
use serana_tools::ToolRegistry;

use crate::{
    handle_tool_turn, validate_message_alternation, AgentLifecycle, AgentRunState, CompressionGate,
    CompressionGateOutcome, ContextCompressor, PromptBuilder, SessionRecorder, ToolCallValidator,
    TurnRunner,
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
}

pub struct AgentEngine<'a> {
    parts: AgentEngineParts<'a>,
}

impl<'a> AgentEngine<'a> {
    pub fn new(parts: AgentEngineParts<'a>) -> Self {
        Self { parts }
    }

    pub async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
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
            let response = TurnRunner::new(self.parts.llm, self.parts.callbacks)
                .run(&messages_snapshot, &tools_snapshot)
                .await?;
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
                    )
                    .await;
                    for tool_call in &turn.tool_calls {
                        self.parts.session_recorder.save_tool_call(tool_call)?;
                    }
                    state.apply_tool_turn(turn);

                    if lifecycle.complete_tool_iteration() {
                        break;
                    }
                }
                Message::Text { role: _, content } => {
                    self.parts
                        .session_recorder
                        .save_message("assistant", &content)?;
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
