use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::config::LlmConfig;
use crate::llm::{LlmClient, Message};
use crate::Result;

pub struct OpenAiClient {
    client: Client,
    config: LlmConfig,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
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
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request_body)
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        let content = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response from OpenAI"))?
            .to_string();

        Ok(content)
    }
}
