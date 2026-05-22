use async_trait::async_trait;
use std::sync::Arc;

use serana_core::{LlmClient, Message, Result, ToolDefinition};

/// Task types that can be delegated to the auxiliary client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxiliaryTask {
    Compression,
    ToolValidation,
    TitleGeneration,
    CodeReview,
    Custom,
}

/// Configuration for auxiliary client.
#[derive(Debug, Clone)]
pub struct AuxiliaryConfig {
    pub max_tokens: usize,
    pub use_fast_model: bool,
    pub timeout_secs: u64,
}

impl Default for AuxiliaryConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            use_fast_model: true,
            timeout_secs: 30,
        }
    }
}

/// Auxiliary LLM client for background tasks.
pub struct AuxiliaryClient {
    inner: Arc<dyn LlmClient>,
    config: AuxiliaryConfig,
}

impl AuxiliaryClient {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self {
            inner: client,
            config: AuxiliaryConfig::default(),
        }
    }

    pub fn with_config(client: Arc<dyn LlmClient>, config: AuxiliaryConfig) -> Self {
        Self {
            inner: client,
            config,
        }
    }

    /// Summarize text for context compression.
    pub async fn summarize(&self, text: &str, max_length: usize) -> Result<String> {
        let prompt = format!(
            "Summarize the following text in at most {} characters. Preserve key information:\n\n{}",
            max_length, text
        );

        let messages = vec![Message::user(prompt)];

        self.chat(&messages).await
    }

    /// Generate a title for a conversation.
    pub async fn generate_title(&self, first_message: &str) -> Result<String> {
        let prompt = format!(
            "Generate a short, descriptive title (max 50 chars) for a conversation that starts with:\n\n{}",
            first_message.chars().take(200).collect::<String>()
        );

        let messages = vec![Message::user(prompt)];
        let title = self.chat(&messages).await?;

        let title = title.lines().next().unwrap_or("").to_string();
        Ok(title.chars().take(50).collect())
    }

    /// Validate a tool call for safety/correctness.
    pub async fn validate_tool_call(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<bool> {
        let prompt = format!(
            "Is this tool call safe to execute? Respond with only 'yes' or 'no'.\n\nTool: {}\nArguments: {}",
            tool_name,
            serde_json::to_string_pretty(arguments)?
        );

        let messages = vec![Message::user(prompt)];
        let response = self.chat(&messages).await?;

        Ok(response.to_lowercase().starts_with("yes"))
    }

    /// Review code for issues.
    pub async fn review_code(&self, code: &str, context: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Review this code for issues. List any problems found, one per line.\n\nContext: {}\n\nCode:\n{}",
            context, code
        );

        let messages = vec![Message::user(prompt)];
        let response = self.chat(&messages).await?;

        Ok(response
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect())
    }
}

#[async_trait]
impl LlmClient for AuxiliaryClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            self.inner.chat(messages),
        )
        .await??;

        let truncated = if result.len() > self.config.max_tokens * 4 {
            result.chars().take(self.config.max_tokens * 4).collect()
        } else {
            result
        };

        Ok(truncated)
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<Message> {
        let response = self.chat(messages).await?;
        Ok(Message::assistant(response))
    }
}

/// Builder for creating auxiliary clients with different configurations.
pub struct AuxiliaryBuilder {
    primary_client: Arc<dyn LlmClient>,
    fast_client: Option<Arc<dyn LlmClient>>,
    config: AuxiliaryConfig,
}

impl AuxiliaryBuilder {
    pub fn new(primary_client: Arc<dyn LlmClient>) -> Self {
        Self {
            primary_client,
            fast_client: None,
            config: AuxiliaryConfig::default(),
        }
    }

    /// Set a separate fast/cheap model client.
    pub fn with_fast_model(mut self, client: Arc<dyn LlmClient>) -> Self {
        self.fast_client = Some(client);
        self
    }

    /// Set custom configuration.
    pub fn with_config(mut self, config: AuxiliaryConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the auxiliary client.
    pub fn build(self) -> AuxiliaryClient {
        let client = if self.config.use_fast_model {
            self.fast_client.unwrap_or(self.primary_client)
        } else {
            self.primary_client
        };

        AuxiliaryClient::with_config(client, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("Mock response".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("Mock response".to_string()))
        }
    }

    #[tokio::test]
    async fn generates_title() {
        let client = AuxiliaryClient::new(Arc::new(MockLlm));
        let title = client
            .generate_title("How do I implement OAuth?")
            .await
            .unwrap();
        assert!(!title.is_empty());
    }

    #[tokio::test]
    async fn summarizes_text() {
        let client = AuxiliaryClient::new(Arc::new(MockLlm));
        let summary = client.summarize("Long text here", 100).await.unwrap();
        assert!(!summary.is_empty());
    }

    #[tokio::test]
    async fn builder_creates_client() {
        let client = AuxiliaryBuilder::new(Arc::new(MockLlm)).build();

        let result = client.chat(&[]).await.unwrap();
        assert_eq!(result, "Mock response");
    }
}
