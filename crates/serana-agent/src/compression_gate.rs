use std::sync::Arc;

use serana_core::{CompressionDecision, LlmClient, Message, Result};
use serana_llm::AuxiliaryClient;

use crate::ContextCompressor;

pub enum CompressionGateOutcome {
    Compressed(Vec<Message>),
    Unchanged,
}

pub struct CompressionGate<'a> {
    compressor: &'a ContextCompressor,
    llm: &'a dyn LlmClient,
    auxiliary: Option<Arc<AuxiliaryClient>>,
}

impl<'a> CompressionGate<'a> {
    pub fn new(
        compressor: &'a ContextCompressor,
        llm: &'a dyn LlmClient,
        auxiliary: Option<Arc<AuxiliaryClient>>,
    ) -> Self {
        Self {
            compressor,
            llm,
            auxiliary,
        }
    }

    pub async fn check(&self, messages: &[Message]) -> Result<CompressionGateOutcome> {
        match self
            .compressor
            .check_compression(self.compressor.estimate_tokens(messages))
        {
            CompressionDecision::Gateway => {
                let compressed = if let Some(aux) = &self.auxiliary {
                    self.compressor
                        .compress_messages_with_auxiliary(messages, aux.as_ref())
                        .await?
                } else {
                    self.compressor
                        .compress_messages(messages, self.llm)
                        .await?
                };
                Ok(CompressionGateOutcome::Compressed(compressed))
            }
            CompressionDecision::Preflight | CompressionDecision::None => {
                Ok(CompressionGateOutcome::Unchanged)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serana_core::{CompressionConfig, CompressionThresholds, ToolDefinition};

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("compressed context".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("unused".to_string()))
        }
    }

    #[tokio::test]
    async fn returns_unchanged_before_gateway_threshold() {
        let compressor = ContextCompressor::new(CompressionConfig {
            max_tokens: 1000,
            protect_last_n: 1,
            thresholds: CompressionThresholds {
                preflight: 0.5,
                gateway: 1.0,
            },
        });
        let gate = CompressionGate::new(&compressor, &MockLlm, None);

        let outcome = gate
            .check(&[Message::user("short".to_string())])
            .await
            .unwrap();

        assert!(matches!(outcome, CompressionGateOutcome::Unchanged));
    }

    #[tokio::test]
    async fn compresses_at_gateway_threshold() {
        let compressor = ContextCompressor::new(CompressionConfig {
            max_tokens: 1,
            protect_last_n: 1,
            thresholds: CompressionThresholds {
                preflight: 0.0,
                gateway: 0.0,
            },
        });
        let gate = CompressionGate::new(&compressor, &MockLlm, None);
        let messages = vec![
            Message::system("system".to_string()),
            Message::user("old".to_string()),
            Message::assistant("old response".to_string()),
            Message::user("latest".to_string()),
        ];

        let outcome = gate.check(&messages).await.unwrap();

        match outcome {
            CompressionGateOutcome::Compressed(messages) => {
                assert!(messages.iter().any(|message| {
                    matches!(message, Message::Text { content, .. } if content.contains("Previous conversation summary"))
                }));
            }
            CompressionGateOutcome::Unchanged => panic!("expected compression"),
        }
    }
}
