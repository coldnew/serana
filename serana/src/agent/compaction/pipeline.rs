use crate::core::{CompactionAction, CompactionConfig, CompactionResult, CompactionStats, ToolTokenDetail, Message};

use crate::agent::compressor::estimate_message_tokens;

use super::accounting::{build_stats, StatsInput};
use super::pass::{Pass, PassContext};
use super::passes::{Collapse, Evict, Microcompact, Reclaim, Shrink};
use super::pressure::Pressure;
use super::sanitize::sanitize_tool_pairs;

fn default_pipeline() -> Vec<Box<dyn Pass>> {
    vec![
        Box::new(Reclaim),
        Box::new(Shrink),
        Box::new(Microcompact),
        Box::new(Collapse),
        Box::new(Evict),
    ]
}

pub fn compact_messages(
    messages: Vec<Message>,
    config: &CompactionConfig,
) -> CompactionResult {
    let passes = default_pipeline();
    let before_message_count = messages.len();
    let before_tool_details = collect_tool_details(&messages);

    let mut current_messages = messages;
    let before_estimated_tokens = estimate_message_tokens(&current_messages);
    let mut all_actions: Vec<CompactionAction> = Vec::new();
    let mut max_level: u8 = 0;

    for pass in &passes {
        let pressure = Pressure::from_messages(&current_messages, config);
        let ctx = PassContext { config, pressure };

        if !pass.should_run(&ctx) {
            continue;
        }

        let result = pass.run(current_messages, &ctx);
        let had_actions = !result.actions.is_empty();
        if had_actions {
            max_level = max_level.max(pass.level() as u8);
        }

        all_actions.extend(result.actions);
        current_messages = result.messages;
    }

    current_messages = sanitize_tool_pairs(current_messages);

    let after_message_count = current_messages.len();
    let after_tool_details = collect_tool_details(&current_messages);
    let after_estimated_tokens = estimate_message_tokens(&current_messages);

    let stats = build_stats(StatsInput {
        level: max_level,
        before_message_count,
        after_message_count,
        before_estimated_tokens,
        after_estimated_tokens,
        before_tool_details,
        after_tool_details,
        actions: all_actions,
    });

    CompactionResult {
        messages: current_messages,
        stats,
    }
}

fn collect_tool_details(messages: &[Message]) -> Vec<ToolTokenDetail> {
    let mut details = Vec::new();
    for msg in messages {
        if let Message::ToolResult { content, .. } = msg {
            details.push(ToolTokenDetail {
                tool_name: "tool".to_string(),
                tokens: (content.len() + 4) / 4,
            });
        }
    }
    details.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    details
}
