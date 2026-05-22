//! Anthropic Messages API client.

use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

use serana_core::{Config, FunctionCall, LlmClient, Message, ToolCallData, ToolDefinition};
use serana_core::Result;

use crate::streaming::SseStream;

/// Anthropic Messages API client.
pub struct AnthropicClient {
    client: Client,
    config: Config,
}

impl AnthropicClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn api_key(&self) -> Result<String> {
        self.config
            .api_key()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("No Anthropic API key configured"))
    }

    fn api_url(&self) -> String {
        format!("{}/messages", self.config.api_url())
    }

    fn convert_messages(messages: &[Message]) -> (String, Vec<serde_json::Value>) {
        let mut system = String::new();
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg {
                Message::Text { role, content } => {
                    match role.as_str() {
                        "system" => {
                            if !system.is_empty() {
                                system.push('\n');
                            }
                            system.push_str(content);
                        }
                        "user" => {
                            api_messages.push(json!({
                                "role": "user",
                                "content": content
                            }));
                        }
                        "assistant" => {
                            api_messages.push(json!({
                                "role": "assistant",
                                "content": content
                            }));
                        }
                        _ => {}
                    }
                }
                Message::ToolCall { role: _, content, tool_calls } => {
                    let mut blocks = Vec::new();
                    if let Some(text) = content {
                        blocks.push(json!({"type": "text", "text": text}));
                    }
                    for tc in tool_calls {
                        let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                        blocks.push(json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.function.name,
                            "input": args
                        }));
                    }
                    api_messages.push(json!({
                        "role": "assistant",
                        "content": blocks
                    }));
                }
                Message::ToolResult { tool_call_id, content, .. } => {
                    api_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": content
                        }]
                    }));
                }
            }
        }

        (system, api_messages)
    }

    fn convert_tools(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "input_schema": t.function.parameters
                })
            })
            .collect()
    }

    fn parse_response(body: &serde_json::Value) -> Result<Message> {
        let content_blocks = body["content"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing content in Anthropic response"))?;

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in content_blocks {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(text) = block["text"].as_str() {
                        text_parts.push(text.to_string());
                    }
                }
                Some("tool_use") => {
                    tool_calls.push(ToolCallData {
                        id: block["id"].as_str().unwrap_or("").to_string(),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: block["name"].as_str().unwrap_or("").to_string(),
                            arguments: serde_json::to_string(
                                block.get("input").unwrap_or(&serde_json::Value::Null),
                            )
                            .unwrap_or_default(),
                        },
                    });
                }
                _ => {}
            }
        }

        let text = text_parts.join("");

        if !tool_calls.is_empty() {
            Ok(Message::ToolCall {
                role: "assistant".to_string(),
                content: if text.is_empty() { None } else { Some(text) },
                tool_calls,
            })
        } else {
            Ok(Message::Text {
                role: "assistant".to_string(),
                content: text,
            })
        }
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let msg = self.chat_with_tools(messages, &[]).await?;
        match msg {
            Message::Text { content, .. } => Ok(content),
            Message::ToolCall { content, .. } => Ok(content.unwrap_or_default()),
            _ => Ok(String::new()),
        }
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let (system, api_messages) = Self::convert_messages(messages);
        let api_tools = Self::convert_tools(tools);

        let mut body = json!({
            "model": self.config.model(),
            "max_tokens": 8192,
            "messages": api_messages,
        });

        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !api_tools.is_empty() {
            body["tools"] = json!(api_tools);
        }

        let resp = self
            .client
            .post(&self.api_url())
            .header("x-api-key", self.api_key()?)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await?;
            anyhow::bail!("Anthropic API error ({}): {}", status, err_body);
        }

        let resp_body: serde_json::Value = resp.json().await?;
        Self::parse_response(&resp_body)
    }

    fn chat_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> Pin<Box<dyn Stream<Item = Result<String>> + Send + 'a>> {
        let client = self.client.clone();
        let config = self.config.clone();
        let api_key = self.api_key().ok();
        let messages_owned = messages.to_vec();

        Box::pin(async_stream::stream! {
            let (system, api_messages) = AnthropicClient::convert_messages(&messages_owned);

            let mut body = json!({
                "model": config.model(),
                "max_tokens": 8192,
                "messages": api_messages,
                "stream": true,
            });

            if !system.is_empty() {
                body["system"] = json!(system);
            }

            let key = api_key.ok_or_else(|| anyhow::anyhow!("No Anthropic API key"))?;

            let url = format!("{}/messages", config.api_url());
            let request = client
                .post(&url)
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body);

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let err_body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("Anthropic API error ({}): {}", status, err_body));
                        return;
                    }
                    let mut sse_stream = SseStream::new(response);
                    while let Some(chunk) = sse_stream.next().await {
                        match chunk {
                            Ok(json_str) => {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    // Anthropic streaming: content_block_delta events
                                    if json["type"].as_str() == Some("content_block_delta") {
                                        if let Some(text) = json["delta"]["text"].as_str() {
                                            yield Ok(text.to_string());
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    }
                }
                Err(e) => yield Err(e.into()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_system_messages() {
        let messages = vec![
            Message::system("You are helpful".to_string()),
            Message::user("Hello".to_string()),
        ];
        let (system, api_messages) = AnthropicClient::convert_messages(&messages);
        assert_eq!(system, "You are helpful");
        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0]["role"], "user");
    }

    #[test]
    fn converts_tool_calls() {
        let tools = vec![ToolDefinition {
            r#type: "function".to_string(),
            function: serana_core::FunctionDefinition {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
            },
        }];
        let api_tools = AnthropicClient::convert_tools(&tools);
        assert_eq!(api_tools.len(), 1);
        assert_eq!(api_tools[0]["name"], "read_file");
    }

    #[test]
    fn parses_text_response() {
        let body = json!({
            "content": [{"type": "text", "text": "Hello world"}]
        });
        let msg = AnthropicClient::parse_response(&body).unwrap();
        match msg {
            Message::Text { content, .. } => assert_eq!(content, "Hello world"),
            _ => panic!("Expected Text message"),
        }
    }

    #[test]
    fn parses_tool_use_response() {
        let body = json!({
            "content": [{
                "type": "tool_use",
                "id": "call_1",
                "name": "read_file",
                "input": {"path": "test.rs"}
            }]
        });
        let msg = AnthropicClient::parse_response(&body).unwrap();
        match msg {
            Message::ToolCall { tool_calls, .. } => {
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].function.name, "read_file");
            }
            _ => panic!("Expected ToolCall message"),
        }
    }
}
