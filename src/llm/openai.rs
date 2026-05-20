//! OpenAI-compatible LLM provider
//!
//! Supports both OpenAI and OpenAI-compatible APIs (e.g., Azure, local proxies, other providers)

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::config::Config;
use crate::llm::{LlmClient, Message, ToolCallData, FunctionCall, ToolDefinition};
use crate::Result;

/// Chat completion response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

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

    /// Send a chat completion request with tool support
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

        let response_body: serde_json::Value = response.json().await?;
        
        // Parse the response into a Message enum
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
        let url = format!("{}/chat/completions", self.config.api_url());

        let request_body = json!({
            "model": self.config.model(),
            "messages": messages,
            "temperature": self.config.temperature(),
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

        let response_body: ChatResponse = response.json().await?;

        let content = response_body.choices.first()
            .and_then(|c| c.message.content.as_deref())
            .ok_or_else(|| anyhow::anyhow!("Empty response from LLM API"))?;

        Ok(content.to_string())
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        self.chat_with_tools_impl(messages, tools).await
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
