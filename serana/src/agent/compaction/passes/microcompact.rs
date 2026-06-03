use crate::core::{CompactionAction, CompactionMethod, Message};

use crate::agent::compressor::estimate_message_tokens;
use super::super::pass::{Pass, PassContext, PassLevel, PassResult};

/// Microcompact pass — token-budget-driven tool result clearing.
///
/// Keeps the most recent compactable tool results whose cumulative tokens
/// fit within the microcompact token budget. Older results are replaced
/// with metadata stubs.
pub struct Microcompact;

impl Pass for Microcompact {
    fn level(&self) -> PassLevel {
        PassLevel::Microcompact
    }

    fn should_run(&self, ctx: &PassContext<'_>) -> bool {
        ctx.pressure.estimated_tokens > ctx.config.compact_trigger()
    }

    fn run(&self, messages: Vec<Message>, ctx: &PassContext<'_>) -> PassResult {
        let config = ctx.config;
        let len = messages.len();
        let recent_boundary = len.saturating_sub(config.keep_recent);

        let tool_result_indices: Vec<usize> = messages
            .iter()
            .enumerate()
            .take(recent_boundary)
            .filter_map(|(i, msg)| {
                if matches!(msg, Message::ToolResult { .. }) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        if tool_result_indices.is_empty() {
            return PassResult { messages, actions: vec![] };
        }

        let mut result = messages;
        let mut actions = Vec::new();
        let mut budget_remaining = config.microcompact_keep_tokens;
        let mut cleared_count = 0usize;

        for &idx in tool_result_indices.iter().rev() {
            if let Message::ToolResult { content, .. } = &result[idx] {
                let tokens = (content.len() + 4) / 4;
                if budget_remaining >= tokens {
                    budget_remaining -= tokens;
                    continue;
                }
            }

            if let Message::ToolResult { tool_call_id, content, .. } = &result[idx] {
                let before_tokens = (content.len() + 4) / 4;
                let replacement = format!("[tool result cleared — {before_tokens} tokens]");
                let after_tokens = (replacement.len() + 4) / 4;

                if after_tokens >= before_tokens {
                    continue;
                }

                actions.push(CompactionAction {
                    index: idx,
                    tool_name: "tool".to_string(),
                    method: CompactionMethod::AgeCleared,
                    before_tokens,
                    after_tokens,
                    end_index: None,
                    related_count: None,
                });

                result[idx] = Message::tool_result(tool_call_id.clone(), replacement);
                cleared_count += 1;
            }
        }

        PassResult { messages: result, actions }
    }
}
