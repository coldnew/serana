use crate::core::{CompactionConfig, Message};

use crate::agent::compressor::estimate_message_tokens;

#[derive(Debug, Clone, Copy)]
pub struct Pressure {
    pub message_tokens: usize,
    pub estimated_tokens: usize,
    pub message_count: usize,
}

impl Pressure {
    pub fn from_messages(messages: &[Message], config: &CompactionConfig) -> Self {
        let message_tokens = estimate_message_tokens(messages);
        let estimated_tokens = message_tokens;
        let message_count = messages.len();

        Self {
            message_tokens,
            estimated_tokens,
            message_count,
        }
    }

    pub fn effective_tokens(&self) -> usize {
        self.estimated_tokens.max(self.message_tokens)
    }
}
