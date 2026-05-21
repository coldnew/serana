use async_trait::async_trait;
use std::path::PathBuf;
use crate::agent::{
    Agent, AgentOutput, AgentCallbacks, AgentStatus,
    IterationBudget, PromptBuilder, execute_tools_concurrent, validate_message_alternation,
};
use crate::llm::{LlmClient, Message, ToolDefinition, FunctionDefinition};
use crate::tools::ToolRegistry;
use crate::Result;

/// Hermes-style coding agent with iteration budget and callback support.
pub struct CodingAgent {
    llm: Box<dyn LlmClient>,
    tools: ToolRegistry,
    budget: IterationBudget,
    callbacks: AgentCallbacks,
    prompt_builder: PromptBuilder,
}

impl CodingAgent {
    pub fn new(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> Self {
        Self {
            llm,
            tools,
            budget: IterationBudget::default_parent(),
            callbacks: AgentCallbacks::new(),
            prompt_builder: PromptBuilder::new(PathBuf::from(".")),
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
}

#[async_trait]
impl Agent for CodingAgent {
    fn name(&self) -> &'static str {
        "coding-agent"
    }

    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        // Notify status
        if let Some(cb) = &self.callbacks.status {
            cb(AgentStatus::Running);
        }

        let system_prompt = self.prompt_builder.build();
        let tools = self.build_tool_definitions();
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(instruction.to_string()),
        ];

        let mut all_tool_calls = Vec::new();

        // Use iteration budget instead of hardcoded limit
        while self.budget.remaining() > 0 {
            // Validate message alternation before each LLM call
            if let Err(e) = validate_message_alternation(&messages) {
                anyhow::bail!("Message alternation error: {}", e);
            }

            let response = self.llm.chat_with_tools(&messages, &tools).await?;

            match response {
                Message::ToolCall { role, content, tool_calls } => {
                    // Add assistant message with tool calls to history
                    messages.push(Message::ToolCall { role, content, tool_calls: tool_calls.clone() });

                    // Execute tools
                    let results = execute_tools_concurrent(&tool_calls, &self.tools, &self.callbacks).await;

                    // Process results
                    for result in results {
                        let tool_call = result.to_tool_call();
                        let result_str = result.result_string();
                        messages.push(Message::tool_result(result.id, result_str));
                        all_tool_calls.push(tool_call);
                    }

                    // Check if budget exceeded after iteration
                    if self.budget.increment() {
                        if let Some(cb) = &self.callbacks.status {
                            cb(AgentStatus::BudgetExhausted);
                        }
                        break;
                    }
                }
                Message::Text { role: _, content } => {
                    // Final response - no more tool calls
                    if let Some(cb) = &self.callbacks.status {
                        cb(AgentStatus::Complete);
                    }
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

        if let Some(cb) = &self.callbacks.status {
            cb(AgentStatus::BudgetExhausted);
        }

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
}
