//! Fallback provider chain for resilient LLM access.
//!
//! Tries providers in order, falling back on failures.

use crate::llm::{LlmClient, Message, ToolDefinition};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Provider status for health tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Healthy,
    Degraded,
    Failed,
}

/// Provider in the fallback chain.
pub struct ProviderEntry {
    pub name: String,
    pub client: Arc<dyn LlmClient>,
    pub status: ProviderStatus,
    pub consecutive_failures: u32,
}

/// Fallback chain configuration.
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Maximum consecutive failures before marking provider as failed
    pub max_failures: u32,
    /// Whether to retry failed providers
    pub retry_failed: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            max_failures: 3,
            retry_failed: true,
        }
    }
}

/// Fallback provider chain.
pub struct FallbackChain {
    providers: Vec<ProviderEntry>,
    config: FallbackConfig,
}

impl FallbackChain {
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            providers: Vec::new(),
            config,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FallbackConfig::default())
    }

    /// Add a provider to the chain.
    pub fn add_provider(&mut self, name: impl Into<String>, client: Arc<dyn LlmClient>) {
        self.providers.push(ProviderEntry {
            name: name.into(),
            client,
            status: ProviderStatus::Healthy,
            consecutive_failures: 0,
        });
    }

    /// Get the current active provider.
    pub fn active_provider(&self) -> Option<&ProviderEntry> {
        self.providers
            .iter()
            .find(|p| p.status != ProviderStatus::Failed)
    }

    /// Mark a provider as failed.
    pub fn mark_failed(&mut self, name: &str) {
        if let Some(provider) = self.providers.iter_mut().find(|p| p.name == name) {
            provider.consecutive_failures += 1;
            if provider.consecutive_failures >= self.config.max_failures {
                provider.status = ProviderStatus::Failed;
            } else {
                provider.status = ProviderStatus::Degraded;
            }
        }
    }

    /// Mark a provider as healthy.
    pub fn mark_healthy(&mut self, name: &str) {
        if let Some(provider) = self.providers.iter_mut().find(|p| p.name == name) {
            provider.consecutive_failures = 0;
            provider.status = ProviderStatus::Healthy;
        }
    }

    /// Get provider count.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get healthy provider count.
    pub fn healthy_count(&self) -> usize {
        self.providers.iter().filter(|p| p.status == ProviderStatus::Healthy).count()
    }
}

#[async_trait]
impl LlmClient for FallbackChain {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let mut last_error = None;

        for provider in &self.providers {
            if provider.status == ProviderStatus::Failed && !self.config.retry_failed {
                continue;
            }

            match provider.client.chat(messages).await {
                Ok(response) => {
                    // Mark as healthy on success
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Continue to next provider
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No providers available")))
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let mut last_error = None;

        for provider in &self.providers {
            if provider.status == ProviderStatus::Failed && !self.config.retry_failed {
                continue;
            }

            match provider.client.chat_with_tools(messages, tools).await {
                Ok(response) => {
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No providers available")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        should_fail: bool,
    }

    #[async_trait]
    impl LlmClient for MockProvider {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("Provider failed"))
            } else {
                Ok("Response".to_string())
            }
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            if self.should_fail {
                Err(anyhow::anyhow!("Provider failed"))
            } else {
                Ok(Message::assistant("Response".to_string()))
            }
        }
    }

    #[tokio::test]
    async fn falls_back_on_failure() {
        let mut chain = FallbackChain::with_defaults();
        chain.add_provider("fail", Arc::new(MockProvider { should_fail: true }));
        chain.add_provider("success", Arc::new(MockProvider { should_fail: false }));

        let response = chain.chat(&[]).await.unwrap();
        assert_eq!(response, "Response");
    }

    #[tokio::test]
    async fn returns_error_when_all_fail() {
        let mut chain = FallbackChain::with_defaults();
        chain.add_provider("fail1", Arc::new(MockProvider { should_fail: true }));
        chain.add_provider("fail2", Arc::new(MockProvider { should_fail: true }));

        let result = chain.chat(&[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn tracks_provider_status() {
        let mut chain = FallbackChain::with_defaults();
        chain.add_provider("test", Arc::new(MockProvider { should_fail: false }));

        assert_eq!(chain.healthy_count(), 1);

        chain.mark_failed("test");
        assert_eq!(chain.providers[0].status, ProviderStatus::Degraded);
        assert_eq!(chain.providers[0].consecutive_failures, 1);

        // After 3 failures, marked as Failed
        chain.mark_failed("test");
        chain.mark_failed("test");
        assert_eq!(chain.providers[0].status, ProviderStatus::Failed);

        chain.mark_healthy("test");
        assert_eq!(chain.providers[0].status, ProviderStatus::Healthy);
    }
}
