//! OpenAI-compatible LLM provider with SSE streaming support

use async_trait::async_trait;
use futures::stream::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

use crate::config::Config;
use crate::llm::{LlmClient, Message, ToolCallData, FunctionCall, ToolDefinition, SseStream};
use crate::Result;


/// OpenAI-compatible LLM client
pub struct OpenAiClient {
    client: Client,
    config: Config,
}

impl OpenAiClient {
    /// Create a new OpenAI-compatible client
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a client with custom HTTP client
    pub fn with_client(client: Client, config: Config) -> Self {
        Self { client, config }
    }

    /// Build the authorization header value
    fn auth_header(&self) -> Option<String> {
        self.config.api_key().map(|k| format!("Bearer {}", k))
    }

    /// Send a chat completion request with tool support (internal, non-streaming)
    pub async fn chat_with_tools_impl(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let url = format!("{}/chat/completions", self.config.api_url());
        
        let request_body = json!({
            "model": self.config.model(),
            "messages": messages,
            "temperature": self.config.temperature(),
            "tools": tools,
        });

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body);
        
        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            anyhow::bail!("LLM API error ({}): {}", status, body);
        }

        let response_text = response.text().await?;
        tracing::debug!("LLM API response: {}", response_text);

        // Handle SSE streaming responses
        if response_text.contains("data:") {
            return parse_streaming_tool_response(&response_text);
        }

        let response_body: serde_json::Value = serde_json::from_str(&response_text)?;
        
        let msg = if let Some(tool_calls) = response_body["choices"][0]["message"]["tool_calls"].as_array() {
            let mut tc = Vec::new();
            for call in tool_calls {
                tc.push(ToolCallData {
                    id: call["id"].as_str().unwrap_or("").to_string(),
                    r#type: call["type"].as_str().unwrap_or("function").to_string(),
                    function: FunctionCall {
                        name: call["function"]["name"].as_str().unwrap_or("").to_string(),
                        arguments: call["function"]["arguments"].as_str().unwrap_or("").to_string(),
                    },
                });
            }
            let content = response_body["choices"][0]["message"]["content"].as_str().map(|s| s.to_string());
            Message::ToolCall {
                role: "assistant".to_string(),
                content,
                tool_calls: tc,
            }
        } else {
            let content = response_body["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
            Message::Text {
                role: "assistant".to_string(),
                content,
            }
        };

        Ok(msg)
    }
}

/// Parse SSE streaming response that may contain tool calls
fn parse_streaming_tool_response(response_text: &str) -> Result<Message> {
    let mut content = String::new();
    let mut tool_calls: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new();
    
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
        
        // Check for tool calls in delta
        if let Some(delta_tool_calls) = chunk["choices"][0]["delta"]["tool_calls"].as_array() {
            for call in delta_tool_calls {
                let index = call["index"].as_u64().unwrap_or(0) as usize;
                let entry = tool_calls.entry(index).or_insert_with(|| {
                    (
                        String::new(), // id
                        String::new(), // type
                        String::new(), // function name
                    )
                });
                
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
        
        // Check for content in delta
        if let Some(delta_content) = chunk["choices"][0]["delta"]["content"].as_str() {
            content.push_str(delta_content);
        }
        
        // Check for message content (non-delta)
        if let Some(msg_content) = chunk["choices"][0]["message"]["content"].as_str() {
            content.push_str(msg_content);
        }
    }
    
    // If we have tool calls, return ToolCall message
    if !tool_calls.is_empty() {
        let tc: Vec<ToolCallData> = tool_calls.into_iter().map(|(_, (id, _t, name))| {
            ToolCallData {
                id,
                r#type: "function".to_string(),
                function: FunctionCall {
                    name,
                    arguments: "{}".to_string(), // Arguments are accumulated separately
                },
            }
        }).collect();
        
        let content = if content.is_empty() { None } else { Some(content) };
        return Ok(Message::ToolCall {
            role: "assistant".to_string(),
            content,
            tool_calls: tc,
        });
    }
    
    // Otherwise return text message
    Ok(Message::Text {
        role: "assistant".to_string(),
        content,
    })
}

#[async_trait]
impl LlmClient for OpenAiClient {
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
        let auth_header = self.auth_header();
        let messages_owned = messages.to_vec();
        
        Box::pin(async_stream::stream! {
            let url = format!("{}/chat/completions", config.api_url());
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
            
            if let Some(auth) = auth_header {
                request = request.header("Authorization", auth);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("LLM API error ({}): {}", status, body));
                        return;
                    }
                    let mut sse_stream = SseStream::new(response);
                    while let Some(chunk) = sse_stream.next().await {
                        match chunk {
                            Ok(json_str) => {
                                // Parse JSON and extract content
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
        let auth_header = self.auth_header();
        let messages_owned = messages.to_vec();
        let tools_owned = tools.to_vec();
        
        Box::pin(async_stream::stream! {
            let url = format!("{}/chat/completions", config.api_url());
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
            
            if let Some(auth) = auth_header {
                request = request.header("Authorization", auth);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("LLM API error ({}): {}", status, body));
                        return;
                    }
                    
                    // Accumulate streaming tool call data
                    let mut tool_call_accumulator: std::collections::HashMap<usize, ToolCallAccumulator> = std::collections::HashMap::new();
                    let mut content_accumulator = String::new();
                    let mut chunk_count = 0;
                    
                    let mut sse_stream = SseStream::new(response);
                    while let Some(chunk) = sse_stream.next().await {
                        match chunk {
                            Ok(json_str) => {
                                chunk_count += 1;
                                // Parse JSON for tool calls
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    // Check choices array
                                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                        if let Some(choice) = choices.get(0) {
                                            if let Some(delta) = choice.get("delta") {
                                                // Handle tool calls in delta
                                                if let Some(tc_array) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                    tracing::debug!("Found {} tool calls in chunk {}", tc_array.len(), chunk_count);
                                                    for tc in tc_array {
                                                        let index = tc["index"].as_u64().unwrap_or(0) as usize;
                                                        let acc = tool_call_accumulator.entry(index).or_default();
                                                        
                                                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                                            acc.id = id.to_string();
                                                            tracing::debug!("Tool call {} id: {}", index, id);
                                                        }
                                                        if let Some(name) = tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
                                                            acc.function_name = name.to_string();
                                                            tracing::debug!("Tool call {} name: {}", index, name);
                                                        }
                                                        if let Some(args) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                                                            acc.function_arguments.push_str(args);
                                                            tracing::debug!("Tool call {} args: {}", index, args);
                                                        }
                                                    }
                                                }
                                                
                                                // Handle content in delta
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
                    
                    tracing::debug!("Stream ended. Processed {} chunks. Tool calls: {}, Content len: {}", 
                        chunk_count, tool_call_accumulator.len(), content_accumulator.len());
                    
                    // Build final message
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
                        yield Err(anyhow::anyhow!("No message received from stream"));
                    }
                }
                Err(e) => yield Err(e.into()),
            }
        })
    }
}

/// Accumulator for streaming tool call data
#[derive(Default)]
struct ToolCallAccumulator {
    id: String,
    function_name: String,
    function_arguments: String,
}

// Clone needed for capturing in streams
impl Clone for OpenAiClient {
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
    fn test_message_serialization() {
        let msg = Message::user("Hello".to_string());
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }
}
