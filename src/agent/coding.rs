use async_trait::async_trait;
use crate::agent::{Agent, AgentOutput, ToolCall};
use crate::llm::{LlmClient, Message};
use crate::tools::{ToolRegistry};
use crate::Result;

pub struct CodingAgent {
    llm: Box<dyn LlmClient>,
    tools: ToolRegistry,
}

impl CodingAgent {
    pub fn new(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> Self {
        Self { llm, tools }
    }
}

#[async_trait]
impl Agent for CodingAgent {
    fn name(&self) -> &'static str {
        "coding-agent"
    }

    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        let system_prompt = self.build_system_prompt();
        let messages = vec![
            Message::system(system_prompt),
            Message::user(instruction.to_string()),
        ];

        let response = self.llm.chat(&messages).await?;

        // Parse response for tool calls (simplified - real impl would parse structured output)
        let tool_calls = self.parse_tool_calls(&response);

        Ok(AgentOutput {
            response,
            tool_calls,
            success: true,
        })
    }

    async fn chat(&self, message: &str) -> Result<String> {
        let messages = vec![Message::user(message.to_string())];
        self.llm.chat(&messages).await
    }
}

impl CodingAgent {
    fn build_system_prompt(&self) -> String {
        let tool_descriptions = self.tools.describe_all();
        format!(
            "You are Serana, a coding agent that helps with programming tasks.\n\n\
             Available tools:\n{}\n\n\
             When you need to perform an action, respond with a tool call in JSON format: \
             {{\"tool\": \"tool_name\", \"arguments\": {{...}}}}",
            tool_descriptions
        )
    }

    fn parse_tool_calls(&self, _response: &str) -> Vec<ToolCall> {
        // Simplified parsing - real implementation would handle structured output
        vec![]
    }
}
