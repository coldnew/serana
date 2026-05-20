//! OpenAI-compatible LLM provider
//!
//! Supports both OpenAI and OpenAI-compatible APIs (e.g., Azure, local proxies, other providers)

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::config::LlmConfig;
use crate::llm::{LlmClient, Message};
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
    config: LlmConfig,
}

impl OpenAiClient {
    /// Create a new OpenAI-compatible client
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a client with custom HTTP client
    pub fn with_client(client: Client, config: LlmConfig) -> Self {
        Self { client, config }
    }

    /// Build the authorization header value
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.api_url);
        
        let request_body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
        });

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

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
