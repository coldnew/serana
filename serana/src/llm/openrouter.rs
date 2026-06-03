//! OpenRouter LLM client.
//!
//! OpenRouter is an OpenAI-compatible gateway that provides access to 100+ models
//! from various providers (Anthropic, Google, Meta, Mistral, etc.) through a single
//! API key. This client adds OpenRouter-specific headers on top of the OpenAI client.

use async_trait::async_trait;
use futures::stream::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

use super::SseStream;
use crate::core::Result;
use crate::core::{Config, FunctionCall, LlmClient, Message, ToolCallData, ToolDefinition};

const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";

pub struct OpenRouterClient {
    client: Client,
    config: Config,
}

impl OpenRouterClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub fn with_client(client: Client, config: Config) -> Self {
        Self { client, config }
    }

    fn api_key(&self) -> Option<String> {
        // Prefer OPENROUTER_API_KEY, fall back to SERANA_API_KEY / config
        std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or_else(|| self.config.api_key())
    }

    fn base_url(&self) -> String {
        self.config
            .provider
            .url
            .clone()
            .unwrap_or_else(|| OPENROUTER_BASE.to_string())
    }

    fn apply_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = self.api_key() {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        request = request.header("HTTP-Referer", "https://github.com/coldnew/serana");
        request = request.header("X-Title", "Serana");
        request
    }

    async fn chat_with_tools_impl(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let url = format!("{}/chat/completions", self.base_url());

        let request_body = json!({
            "model": self.config.model(),
            "messages": messages,
            "temperature": self.config.temperature(),
            "tools": tools,
        });

        let request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body);
        let request = self.apply_headers(request);

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            anyhow::bail!("OpenRouter API error ({}): {}", status, body);
        }

        let response_text = response.text().await?;
        tracing::debug!("OpenRouter response: {}", response_text);

        if response_text.contains("data:") {
            return parse_streaming_tool_response(&response_text);
        }

        let response_body: serde_json::Value = serde_json::from_str(&response_text)?;

        let msg = if let Some(tool_calls) =
            response_body["choices"][0]["message"]["tool_calls"].as_array()
        {
            let mut tc = Vec::new();
            for call in tool_calls {
                tc.push(ToolCallData {
                    id: call["id"].as_str().unwrap_or("").to_string(),
                    r#type: call["type"].as_str().unwrap_or("function").to_string(),
                    function: FunctionCall {
                        name: call["function"]["name"].as_str().unwrap_or("").to_string(),
                        arguments: call["function"]["arguments"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                    },
                });
            }
            let content = response_body["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string());
            Message::ToolCall {
                role: "assistant".to_string(),
                content,
                tool_calls: tc,
            }
        } else {
            let content = response_body["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            Message::Text {
                role: "assistant".to_string(),
                content,
            }
        };

        Ok(msg)
    }
}

fn parse_streaming_tool_response(response_text: &str) -> Result<Message> {
    let mut content = String::new();
    let mut tool_calls: std::collections::HashMap<usize, (String, String, String)> =
        std::collections::HashMap::new();

    for line in response_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" || data.is_empty() {
            continue;
        }

        let chunk: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(delta_tool_calls) = chunk["choices"][0]["delta"]["tool_calls"].as_array() {
            for call in delta_tool_calls {
                let index = call["index"].as_u64().unwrap_or(0) as usize;
                let entry = tool_calls
                    .entry(index)
                    .or_insert_with(|| (String::new(), String::new(), String::new()));

                if let Some(id) = call["id"].as_str() {
                    entry.0 = id.to_string();
                }
                if let Some(t) = call["type"].as_str() {
                    entry.1 = t.to_string();
                }
                if let Some(name) = call["function"]["name"].as_str() {
                    entry.2 = name.to_string();
                }
            }
        }

        if let Some(delta_content) = chunk["choices"][0]["delta"]["content"].as_str() {
            content.push_str(delta_content);
        }
    }

    if !tool_calls.is_empty() {
        let tc: Vec<ToolCallData> = tool_calls
            .into_iter()
            .map(|(_, (id, _t, name))| ToolCallData {
                id,
                r#type: "function".to_string(),
                function: FunctionCall {
                    name,
                    arguments: "{}".to_string(),
                },
            })
            .collect();

        let content = if content.is_empty() {
            None
        } else {
            Some(content)
        };
        return Ok(Message::ToolCall {
            role: "assistant".to_string(),
            content,
            tool_calls: tc,
        });
    }

    Ok(Message::Text {
        role: "assistant".to_string(),
        content,
    })
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let mut stream = self.chat_stream(messages);
        let mut content = String::new();
        while let Some(chunk) = stream.next().await {
            content.push_str(&chunk?);
        }
        Ok(content)
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        self.chat_with_tools_impl(messages, tools).await
    }

    fn chat_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> Pin<Box<dyn Stream<Item = Result<String>> + Send + 'a>> {
        let client = self.client.clone();
        let config = self.config.clone();
        let api_key = self.api_key();
        let base_url = self.base_url();
        let messages_owned = messages.to_vec();

        Box::pin(async_stream::stream! {
            let url = format!("{}/chat/completions", base_url);
            let request_body = json!({
                "model": config.model(),
                "messages": messages_owned,
                "temperature": config.temperature(),
                "stream": true,
            });

            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body);

            if let Some(key) = api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }
            request = request.header("HTTP-Referer", "https://github.com/coldnew/serana");
            request = request.header("X-Title", "Serana");

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("OpenRouter API error ({}): {}", status, body));
                        return;
                    }
                    let mut sse_stream = SseStream::new(response);
                    while let Some(chunk) = sse_stream.next().await {
                        match chunk {
                            Ok(json_str) => {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                        yield Ok(content.to_string());
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

    fn chat_with_tools_stream<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'a>> {
        let client = self.client.clone();
        let config = self.config.clone();
        let api_key = self.api_key();
        let base_url = self.base_url();
        let messages_owned = messages.to_vec();
        let tools_owned = tools.to_vec();

        Box::pin(async_stream::stream! {
            let url = format!("{}/chat/completions", base_url);
            let request_body = json!({
                "model": config.model(),
                "messages": messages_owned,
                "temperature": config.temperature(),
                "stream": true,
                "tools": tools_owned,
            });

            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body);

            if let Some(key) = api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }
            request = request.header("HTTP-Referer", "https://github.com/coldnew/serana");
            request = request.header("X-Title", "Serana");

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("OpenRouter API error ({}): {}", status, body));
                        return;
                    }

                    let mut tool_call_accumulator: std::collections::HashMap<usize, ToolCallAccumulator> = std::collections::HashMap::new();
                    let mut content_accumulator = String::new();

                    let mut sse_stream = SseStream::new(response);
                    while let Some(chunk) = sse_stream.next().await {
                        match chunk {
                            Ok(json_str) => {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                        if let Some(choice) = choices.get(0) {
                                            if let Some(delta) = choice.get("delta") {
                                                if let Some(tc_array) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                    for tc in tc_array {
                                                        let index = tc["index"].as_u64().unwrap_or(0) as usize;
                                                        let acc = tool_call_accumulator.entry(index).or_default();

                                                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                                            acc.id = id.to_string();
                                                        }
                                                        if let Some(name) = tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
                                                            acc.function_name = name.to_string();
                                                        }
                                                        if let Some(args) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                                                            acc.function_arguments.push_str(args);
                                                        }
                                                    }
                                                }

                                                if let Some(c) = delta.get("content").and_then(|c| c.as_str()) {
                                                    content_accumulator.push_str(c);
                                                }
                                            }
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

                    if !tool_call_accumulator.is_empty() {
                        let tool_calls: Vec<ToolCallData> = tool_call_accumulator
                            .into_iter()
                            .map(|(_, acc)| ToolCallData {
                                id: acc.id,
                                r#type: "function".to_string(),
                                function: FunctionCall {
                                    name: acc.function_name,
                                    arguments: acc.function_arguments,
                                },
                            })
                            .collect();

                        let content = if content_accumulator.is_empty() { None } else { Some(content_accumulator) };
                        yield Ok(Message::ToolCall {
                            role: "assistant".to_string(),
                            content,
                            tool_calls,
                        });
                    } else if !content_accumulator.is_empty() {
                        yield Ok(Message::Text {
                            role: "assistant".to_string(),
                            content: content_accumulator,
                        });
                    } else {
                        yield Err(anyhow::anyhow!("No message received from OpenRouter stream"));
                    }
                }
                Err(e) => yield Err(e.into()),
            }
        })
    }
}

#[derive(Default)]
struct ToolCallAccumulator {
    id: String,
    function_name: String,
    function_arguments: String,
}

impl Clone for OpenRouterClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_url() {
        let config = Config::default();
        let client = OpenRouterClient::new(config);
        assert_eq!(client.base_url(), OPENROUTER_BASE);
    }

    #[test]
    fn test_openrouter_custom_url() {
        let mut config = Config::default();
        config.provider.url = Some("https://custom.openrouter.ai/api/v1".to_string());
        let client = OpenRouterClient::new(config);
        assert_eq!(client.base_url(), "https://custom.openrouter.ai/api/v1");
    }
}
