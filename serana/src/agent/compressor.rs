use crate::core::{CompressionConfig, CompressionDecision, LlmClient, Message, Result};
use crate::llm::AuxiliaryClient;

#[derive(Clone)]
pub struct ContextCompressor {
    config: CompressionConfig,
}

impl ContextCompressor {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(CompressionConfig::default())
    }

    pub fn check_compression(&self, current_tokens: usize) -> CompressionDecision {
        let ratio = current_tokens as f32 / self.config.max_tokens as f32;

        if ratio >= self.config.thresholds.gateway {
            CompressionDecision::Gateway
        } else if ratio >= self.config.thresholds.preflight {
            CompressionDecision::Preflight
        } else {
            CompressionDecision::None
        }
    }

    pub async fn compress_messages(
        &self,
        messages: &[Message],
        llm: &dyn LlmClient,
    ) -> Result<Vec<Message>> {
        if messages.len() <= self.config.protect_last_n {
            return Ok(messages.to_vec());
        }

        let (system_msgs, rest) = self.split_system_messages(messages);
        let split_point = rest.len().saturating_sub(self.config.protect_last_n);
        let (to_compress, protected) = rest.split_at(split_point);

        if to_compress.is_empty() {
            return Ok(messages.to_vec());
        }

        let summary = self.summarize_messages(to_compress, llm).await?;

        let mut result = system_msgs;
        result.push(Message::user(format!(
            "[Previous conversation summary]\n{}",
            summary
        )));
        result.extend_from_slice(protected);

        Ok(result)
    }

    pub async fn compress_messages_with_auxiliary(
        &self,
        messages: &[Message],
        auxiliary: &AuxiliaryClient,
    ) -> Result<Vec<Message>> {
        if messages.len() <= self.config.protect_last_n {
            return Ok(messages.to_vec());
        }

        let (system_msgs, rest) = self.split_system_messages(messages);
        let split_point = rest.len().saturating_sub(self.config.protect_last_n);
        let (to_compress, protected) = rest.split_at(split_point);

        if to_compress.is_empty() {
            return Ok(messages.to_vec());
        }

        let conversation = to_compress
            .iter()
            .map(|m| format!("{}: {:?}", m.role(), m))
            .collect::<Vec<_>>()
            .join("\n\n");

        let summary = auxiliary.summarize(&conversation, 2000).await?;

        let mut result = system_msgs;
        result.push(Message::user(format!(
            "[Previous conversation summary]\n{}",
            summary
        )));
        result.extend_from_slice(protected);

        Ok(result)
    }

    fn split_system_messages<'a>(&self, messages: &'a [Message]) -> (Vec<Message>, &'a [Message]) {
        let system_count = messages.iter().take_while(|m| m.role() == "system").count();
        let (system, rest) = messages.split_at(system_count);
        (system.to_vec(), rest)
    }

    async fn summarize_messages(
        &self,
        messages: &[Message],
        llm: &dyn LlmClient,
    ) -> Result<String> {
        let conversation = messages
            .iter()
            .map(|m| format!("{}: {:?}", m.role(), m))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = vec![
            Message::system("You are a conversation summarizer. Produce a concise summary that preserves key decisions, context, and unresolved issues. Focus on technical details, file paths, and action items.".to_string()),
            Message::user(format!("Summarize this conversation:\n\n{}", conversation)),
        ];

        llm.chat(&prompt).await
    }

    pub fn estimate_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| {
                let content_len = match m {
                    Message::Text { content, .. } => content.len(),
                    Message::ToolCall {
                        content,
                        tool_calls,
                        ..
                    } => content.as_ref().map(|c| c.len()).unwrap_or(0) + tool_calls.len() * 50,
                    Message::ToolResult { content, .. } => content.len(),
                };
                (m.role().len() + content_len) / 4
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CompressionThresholds;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("Summary of previous conversation.".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[crate::core::ToolDefinition],
        ) -> Result<Message> {
            unimplemented!()
        }
    }

    #[test]
    fn detects_compression_thresholds() {
        let compressor = ContextCompressor::with_defaults();

        assert_eq!(
            compressor.check_compression(10_000),
            CompressionDecision::None
        );
        assert_eq!(
            compressor.check_compression(70_000),
            CompressionDecision::Preflight
        );
        assert_eq!(
            compressor.check_compression(110_000),
            CompressionDecision::Gateway
        );
    }

    #[test]
    fn estimates_token_count() {
        let compressor = ContextCompressor::with_defaults();
        let messages = vec![
            Message::user("a".repeat(400)),
            Message::assistant("b".repeat(400)),
        ];

        let estimate = compressor.estimate_tokens(&messages);
        assert!(estimate >= 190 && estimate <= 210);
    }

    #[tokio::test]
    async fn protects_recent_messages() {
        let config = CompressionConfig {
            max_tokens: 128_000,
            protect_last_n: 2,
            thresholds: CompressionThresholds::default(),
            ..Default::default()
        };
        let compressor = ContextCompressor::new(config);
        let llm = MockLlm;

        let messages = vec![
            Message::system("System prompt".to_string()),
            Message::user("Old message 1".to_string()),
            Message::assistant("Old response 1".to_string()),
            Message::user("Recent message 1".to_string()),
            Message::assistant("Recent response 1".to_string()),
        ];

        let compressed = compressor.compress_messages(&messages, &llm).await.unwrap();

        assert_eq!(compressed.len(), 4);
        assert_eq!(compressed[0].role(), "system");
        assert!(
            matches!(&compressed[1], Message::Text { content, .. } if content.contains("Previous conversation summary"))
        );
    }

    #[tokio::test]
    async fn preserves_all_system_messages() {
        let compressor = ContextCompressor::with_defaults();
        let llm = MockLlm;

        let messages = vec![
            Message::system("System 1".to_string()),
            Message::system("System 2".to_string()),
            Message::user("User message".to_string()),
        ];

        let compressed = compressor.compress_messages(&messages, &llm).await.unwrap();

        assert!(compressed.iter().filter(|m| m.role() == "system").count() >= 2);
    }

    #[tokio::test]
    async fn compress_with_auxiliary() {
        let config = CompressionConfig {
            max_tokens: 128_000,
            protect_last_n: 1,
            thresholds: CompressionThresholds::default(),
            ..Default::default()
        };
        let compressor = ContextCompressor::new(config);
        let auxiliary = AuxiliaryClient::new(Arc::new(MockLlm));

        let messages = vec![
            Message::system("System".to_string()),
            Message::user("Old message 1".to_string()),
            Message::assistant("Old response 1".to_string()),
            Message::user("Old message 2".to_string()),
            Message::assistant("Recent".to_string()),
        ];

        let compressed = compressor
            .compress_messages_with_auxiliary(&messages, &auxiliary)
            .await
            .unwrap();
        assert_eq!(compressed.len(), 3);
        assert!(compressed.iter().any(|m| matches!(m, Message::Text { content, .. } if content.contains("Previous conversation summary"))));
    }
}
