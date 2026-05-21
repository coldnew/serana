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

    /// Send a chat completion request with tool support (internal)
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

        // Handle SSE streaming responses (fallback for non-streaming call)
        if let Some(content) = extract_stream_content(&response_text)? {
            return Ok(Message::Text {
                role: "assistant".to_string(),
                content,
            });
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
                        yield chunk;
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
        let this = self.clone();
        let messages_owned = messages.to_vec();
        let tools_owned = tools.to_vec();
        
        Box::pin(async_stream::stream! {
            match this.chat_with_tools_impl(&messages_owned, &tools_owned).await {
                Ok(msg) => yield Ok(msg),
                Err(e) => yield Err(e),
            }
        })
    }
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

/// Extract content from SSE streaming response (for non-streaming fallback)
fn extract_stream_content(response_text: &str) -> Result<Option<String>> {
    if !response_text.lines().any(|line| line.trim_start().starts_with("data:")) {
        return Ok(None);
    }

    let mut content = String::new();
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

        let chunk: serde_json::Value = serde_json::from_str(data)?;
        if let Some(delta) = chunk["choices"][0]["delta"]["content"].as_str() {
            content.push_str(delta);
        } else if let Some(message) = chunk["choices"][0]["message"]["content"].as_str() {
            content.push_str(message);
        }
    }

    Ok(Some(content))
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
