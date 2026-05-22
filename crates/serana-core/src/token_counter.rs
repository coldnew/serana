//! Token counting for context budget management.
//!
//! Uses a simple character-based estimation (chars / 4) that matches
//! the existing compression estimator. Can be swapped for BPE later.

use crate::message::Message;

/// Token counter using character-based estimation.
///
/// For English text, ~4 chars per token is a reasonable heuristic.
/// This is the same formula used in `ContextCompressor::estimate_tokens`.
pub struct TokenCounter {
    chars_per_token: usize,
}

impl TokenCounter {
    pub fn new() -> Self {
        Self { chars_per_token: 4 }
    }

    /// Estimate tokens for a single string.
    pub fn count_text(&self, text: &str) -> usize {
        (text.len() + self.chars_per_token - 1) / self.chars_per_token
    }

    /// Estimate tokens for a list of messages.
    pub fn count_messages(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| {
                let content_len = match m {
                    Message::Text { content, .. } => content.len(),
                    Message::ToolCall {
                        content,
                        tool_calls,
                        ..
                    } => {
                        content.as_ref().map(|c| c.len()).unwrap_or(0)
                            + tool_calls.iter().map(|tc| {
                                tc.function.name.len() + tc.function.arguments.len() + 20
                            }).sum::<usize>()
                    }
                    Message::ToolResult { content, .. } => content.len(),
                };
                let role_overhead = 4; // role tag overhead
                (role_overhead + content_len + self.chars_per_token - 1) / self.chars_per_token
            })
            .sum()
    }

    /// Estimate remaining context budget.
    pub fn remaining_budget(&self, max_tokens: usize, used_tokens: usize) -> usize {
        max_tokens.saturating_sub(used_tokens)
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_text_tokens() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count_text("hello"), 2); // 5 chars / 4 = 1.25 -> 2
        assert_eq!(counter.count_text(""), 0);
        assert_eq!(counter.count_text("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn counts_message_tokens() {
        let counter = TokenCounter::new();
        let messages = vec![
            Message::user("hello world".to_string()),
            Message::assistant("hi there".to_string()),
        ];
        let tokens = counter.count_messages(&messages);
        assert!(tokens > 0);
        // 11 + 4 = 15 chars + role overhead
        assert!(tokens >= 4);
    }

    #[test]
    fn remaining_budget() {
        let counter = TokenCounter::new();
        assert_eq!(counter.remaining_budget(100, 30), 70);
        assert_eq!(counter.remaining_budget(100, 100), 0);
        assert_eq!(counter.remaining_budget(100, 150), 0); // saturating sub
    }
}
