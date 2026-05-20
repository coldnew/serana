use async_trait::async_trait;
use crate::agent::{Agent, AgentOutput, ToolCall};
use crate::llm::{LlmClient, Message, ToolDefinition, FunctionDefinition};
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
        let tools = self.build_tool_definitions();
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(instruction.to_string()),
        ];

        let mut all_tool_calls = Vec::new();
        let max_iterations = 10;

        for _ in 0..max_iterations {
            let response = self.llm.chat_with_tools(&messages, &tools).await?;

            match response {
                Message::ToolCall { role, content, tool_calls } => {
                    // Add assistant message with tool calls to history
                    messages.push(Message::ToolCall { role, content, tool_calls: tool_calls.clone() });

                    // Execute each tool call
                    for tc in tool_calls {
                        let result = self.execute_tool(&tc.function.name, &tc.function.arguments).await;
                        let result_str = match result {
                            Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string()),
                            Err(e) => format!("{{\"error\": \"{}\"}}", e),
                        };

                        all_tool_calls.push(ToolCall {
                            name: tc.function.name.clone(),
                            arguments: serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null),
                            result: Some(serde_json::from_str(&result_str).unwrap_or(serde_json::Value::Null)),
                        });

                        // Add tool result to messages
                        messages.push(Message::tool_result(tc.id, result_str));
                    }
                }
                Message::Text { role: _, content } => {
                    // Final response - no more tool calls
                    return Ok(AgentOutput {
                        response: content,
                        tool_calls: all_tool_calls,
                        success: true,
                    });
                }
                Message::ToolResult { .. } => {
                    // Shouldn't happen - tool results come from us, not LLM
                    anyhow::bail!("Unexpected tool result message from LLM");
                }
            }
        }

        anyhow::bail!("Exceeded maximum tool call iterations");
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
            "You are Serana, a coding agent that helps with programming tasks.\\n\\n\
             Available tools:\\n{}\\n\\n\
             When you need to perform an action, use the provided tools. \
             Analyze the results and continue until the task is complete.",
            tool_descriptions
        )
    }

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

    async fn execute_tool(&self, name: &str, arguments: &str) -> Result<serde_json::Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        tool.execute(args).await
    }
}
