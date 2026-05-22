use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use serana_core::{LlmClient, Message, Result, ToolDefinition};

/// Credential provider trait.
#[async_trait]
pub trait CredentialProvider: Send + Sync {
    async fn get_credentials(&self) -> Result<String>;
    async fn refresh(&self) -> Result<String>;
}

/// Simple static credential provider.
pub struct StaticCredential {
    api_key: String,
}

impl StaticCredential {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl CredentialProvider for StaticCredential {
    async fn get_credentials(&self) -> Result<String> {
        Ok(self.api_key.clone())
    }

    async fn refresh(&self) -> Result<String> {
        Ok(self.api_key.clone())
    }
}

/// LLM client wrapper with automatic credential refresh.
pub struct RefreshableClient {
    inner: Arc<dyn LlmClient>,
    credential_provider: Arc<RwLock<Box<dyn CredentialProvider>>>,
    max_retries: u32,
}

impl RefreshableClient {
    pub fn new(
        inner: Arc<dyn LlmClient>,
        credential_provider: Box<dyn CredentialProvider>,
    ) -> Self {
        Self {
            inner,
            credential_provider: Arc::new(RwLock::new(credential_provider)),
            max_retries: 1,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    fn is_auth_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("401")
            || error_str.contains("403")
            || error_str.contains("unauthorized")
            || error_str.contains("forbidden")
            || error_str.contains("invalid api key")
            || error_str.contains("authentication")
    }

    async fn refresh_and_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if Self::is_auth_error(&e) && attempts < self.max_retries {
                        let provider = self.credential_provider.write().await;
                        provider.refresh().await?;
                        attempts += 1;
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}

#[async_trait]
impl LlmClient for RefreshableClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let messages = messages.to_vec();
        self.refresh_and_retry(|| {
            let inner = self.inner.clone();
            let messages = messages.clone();
            Box::pin(async move { inner.chat(&messages).await })
        })
        .await
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        self.refresh_and_retry(|| {
            let inner = self.inner.clone();
            let messages = messages.clone();
            let tools = tools.clone();
            Box::pin(async move { inner.chat_with_tools(&messages, &tools).await })
        })
        .await
    }
}

/// Environment variable credential provider.
pub struct EnvCredential {
    env_var: String,
    current: RwLock<Option<String>>,
}

impl EnvCredential {
    pub fn new(env_var: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
            current: RwLock::new(None),
        }
    }
}

#[async_trait]
impl CredentialProvider for EnvCredential {
    async fn get_credentials(&self) -> Result<String> {
        let current = self.current.read().await;
        if let Some(ref key) = *current {
            return Ok(key.clone());
        }
        drop(current);

        let key = std::env::var(&self.env_var)
            .map_err(|_| anyhow::anyhow!("Environment variable {} not set", self.env_var))?;

        let mut current = self.current.write().await;
        *current = Some(key.clone());
        Ok(key)
    }

    async fn refresh(&self) -> Result<String> {
        let mut current = self.current.write().await;
        *current = None;
        drop(current);
        self.get_credentials().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClient {
        fail_count: std::sync::atomic::AtomicU32,
    }

    #[async_trait]
    impl LlmClient for MockClient {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            let count = self
                .fail_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                Err(anyhow::anyhow!("401 Unauthorized"))
            } else {
                Ok("Success".to_string())
            }
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("Success".to_string()))
        }
    }

    struct MockCredential {
        refresh_count: std::sync::atomic::AtomicU32,
    }

    #[async_trait]
    impl CredentialProvider for MockCredential {
        async fn get_credentials(&self) -> Result<String> {
            Ok("test-key".to_string())
        }

        async fn refresh(&self) -> Result<String> {
            self.refresh_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("refreshed-key".to_string())
        }
    }

    #[test]
    fn detects_auth_errors() {
        assert!(RefreshableClient::is_auth_error(&anyhow::anyhow!(
            "401 Unauthorized"
        )));
        assert!(RefreshableClient::is_auth_error(&anyhow::anyhow!(
            "403 Forbidden"
        )));
        assert!(RefreshableClient::is_auth_error(&anyhow::anyhow!(
            "Invalid API key"
        )));
        assert!(!RefreshableClient::is_auth_error(&anyhow::anyhow!(
            "Network error"
        )));
    }

    #[tokio::test]
    async fn refreshes_on_auth_error() {
        let credential = MockCredential {
            refresh_count: std::sync::atomic::AtomicU32::new(0),
        };
        let client = RefreshableClient::new(
            Arc::new(MockClient {
                fail_count: std::sync::atomic::AtomicU32::new(0),
            }),
            Box::new(credential),
        );

        let result = client.chat(&[]).await.unwrap();
        assert_eq!(result, "Success");
    }

    #[tokio::test]
    async fn static_credential_returns_same_key() {
        let cred = StaticCredential::new("test-key");
        assert_eq!(cred.get_credentials().await.unwrap(), "test-key");
        assert_eq!(cred.refresh().await.unwrap(), "test-key");
    }
}
