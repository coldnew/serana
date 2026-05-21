//! Context compression for managing token budgets.
//!
//! Compresses conversation history when approaching context limits,
//! preserving recent messages and system prompts.

use crate::llm::{LlmClient, Message, AuxiliaryClient};
use crate::Result;

/// Compression trigger thresholds as percentages of max context.
#[derive(Debug, Clone, Copy)]
pub struct CompressionThresholds {
    /// Preflight check threshold (default 50%)
    pub preflight: f32,
    /// Gateway threshold requiring immediate compression (default 85%)
    pub gateway: f32,
}

impl Default for CompressionThresholds {
    fn default() -> Self {
        Self {
            preflight: 0.50,
            gateway: 0.85,
        }
    }
}

/// Context compression configuration.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum context window size in tokens
    pub max_tokens: usize,
    /// Number of recent messages to protect from compression
    pub protect_last_n: usize,
    /// Compression thresholds
    pub thresholds: CompressionThresholds,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 128_000,
            protect_last_n: 10,
            thresholds: CompressionThresholds::default(),
        }
    }
}

/// Compression decision based on current token usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionDecision {
    /// No compression needed
    None,
    /// Compression recommended but not required
    Preflight,
    /// Compression required before next API call
    Gateway,
}

/// Context compressor that manages conversation history.
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

    /// Check if compression is needed based on current token count.
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

    /// Compress messages using LLM summarization.
    ///
    /// Protects the last N messages and system prompts, compresses the middle section.
    pub async fn compress_messages(
        &self,
        messages: &[Message],
        llm: &dyn LlmClient,
    ) -> Result<Vec<Message>> {
        if messages.len() <= self.config.protect_last_n {
            return Ok(messages.to_vec());
        }

        // Split into system, compressible, and protected sections
        let (system_msgs, rest) = self.split_system_messages(messages);
        let split_point = rest.len().saturating_sub(self.config.protect_last_n);
        let (to_compress, protected) = rest.split_at(split_point);

        if to_compress.is_empty() {
            return Ok(messages.to_vec());
        }

        // Generate summary of compressible section
        let summary = self.summarize_messages(to_compress, llm).await?;

        // Reconstruct: system + summary + protected
        let mut result = system_msgs;
        result.push(Message::user(format!("[Previous conversation summary]\n{}", summary)));
        result.extend_from_slice(protected);

        Ok(result)
    }

    /// Compress messages using auxiliary client (Hermes pattern).
    ///
    /// Uses the auxiliary client for background compression tasks.
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

        // Build conversation text
        let conversation = to_compress
            .iter()
            .map(|m| format!("{}: {:?}", m.role(), m))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Use auxiliary client's summarize method
        let summary = auxiliary.summarize(&conversation, 2000).await?;

        let mut result = system_msgs;
        result.push(Message::user(format!("[Previous conversation summary]\n{}", summary)));
        result.extend_from_slice(protected);

        Ok(result)
    }

    /// Split system messages from the rest.
    fn split_system_messages<'a>(&self, messages: &'a [Message]) -> (Vec<Message>, &'a [Message]) {
        let system_count = messages.iter().take_while(|m| m.role() == "system").count();
        let (system, rest) = messages.split_at(system_count);
        (system.to_vec(), rest)
    }

    /// Summarize a slice of messages using the LLM.
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

    /// Estimate token count (rough approximation: 1 token ≈ 4 chars).
    pub fn estimate_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| {
                let content_len = match m {
                    Message::Text { content, .. } => content.len(),
                    Message::ToolCall { content, tool_calls, .. } => {
                        content.as_ref().map(|c| c.len()).unwrap_or(0) + tool_calls.len() * 50
                    }
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
            _tools: &[crate::llm::ToolDefinition],
        ) -> Result<Message> {
            unimplemented!()
        }
    }

    #[test]
    fn detects_compression_thresholds() {
        let compressor = ContextCompressor::with_defaults();
        
        assert_eq!(compressor.check_compression(10_000), CompressionDecision::None);
        assert_eq!(compressor.check_compression(70_000), CompressionDecision::Preflight);
        assert_eq!(compressor.check_compression(110_000), CompressionDecision::Gateway);
    }

    #[test]
    fn estimates_token_count() {
        let compressor = ContextCompressor::with_defaults();
        let messages = vec![
            Message::user("a".repeat(400)), // ~100 tokens
            Message::assistant("b".repeat(400)), // ~100 tokens
        ];
        
        let estimate = compressor.estimate_tokens(&messages);
        assert!(estimate >= 190 && estimate <= 210); // ~200 tokens
    }

    #[tokio::test]
    async fn protects_recent_messages() {
        let config = CompressionConfig {
            max_tokens: 128_000,
            protect_last_n: 2,
            thresholds: CompressionThresholds::default(),
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

        // Should have: system + summary + 2 protected
        assert_eq!(compressed.len(), 4);
        assert_eq!(compressed[0].role(), "system");
        assert!(matches!(&compressed[1], Message::Text { content, .. } if content.contains("Previous conversation summary")));
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

        // Both system messages should be preserved
        assert!(compressed.iter().filter(|m| m.role() == "system").count() >= 2);
    }

    #[tokio::test]
    async fn compress_with_auxiliary() {
        let config = CompressionConfig {
            max_tokens: 128_000,
            protect_last_n: 1,
            thresholds: CompressionThresholds::default(),
        };
        let compressor = ContextCompressor::new(config);
        let auxiliary = AuxiliaryClient::new(Arc::new(MockLlm));

        // Need enough messages to trigger compression (> protect_last_n)
        let messages = vec![
            Message::system("System".to_string()),
            Message::user("Old message 1".to_string()),
            Message::assistant("Old response 1".to_string()),
            Message::user("Old message 2".to_string()),
            Message::assistant("Recent".to_string()),
        ];

        let compressed = compressor.compress_messages_with_auxiliary(&messages, &auxiliary).await.unwrap();
        // Should have: system + summary + 1 protected
        assert_eq!(compressed.len(), 3);
        assert!(compressed.iter().any(|m| matches!(m, Message::Text { content, .. } if content.contains("Previous conversation summary"))));
    }
}
