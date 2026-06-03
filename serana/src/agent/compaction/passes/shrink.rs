use crate::core::{CompactionAction, CompactionMethod, Message};

use crate::agent::compressor::estimate_message_tokens;
use super::super::pass::{Pass, PassContext, PassLevel, PassResult};

/// Shrink pass — budget-gated truncation of oversized tool results and user messages.
pub struct Shrink;

impl Pass for Shrink {
    fn level(&self) -> PassLevel {
        PassLevel::Shrink
    }

    fn should_run(&self, ctx: &PassContext<'_>) -> bool {
        ctx.pressure.estimated_tokens > ctx.config.compact_trigger()
    }

    fn run(&self, messages: Vec<Message>, ctx: &PassContext<'_>) -> PassResult {
        let len = messages.len();
        let recent_boundary = len.saturating_sub(ctx.config.keep_recent);
        let oversize_threshold = oversize_threshold(ctx.config);

        let mut running_tokens = ctx.pressure.estimated_tokens;
        let mut actions = Vec::new();
        let mut result = Vec::with_capacity(len);

        for (idx, msg) in messages.into_iter().enumerate() {
            let is_recent = idx >= recent_boundary;
            let is_pinned = idx < ctx.config.keep_first;

            if let Message::Text { role, content } = &msg {
                if role != "user" || is_pinned || is_recent {
                    result.push(msg);
                    continue;
                }
                let content_tokens = (content.len() + 4) / 4;
                if content_tokens > oversize_threshold {
                    let before_tokens = content_tokens;
                    let truncated = truncate_text_head_tail(content, ctx.config.tool_output_max_lines);
                    let after_tokens = (truncated.len() + 4) / 4;
                    running_tokens = running_tokens.saturating_sub(before_tokens.saturating_sub(after_tokens));
                    actions.push(CompactionAction {
                        index: idx,
                        tool_name: "user".to_string(),
                        method: CompactionMethod::OversizeCapped,
                        before_tokens,
                        after_tokens,
                        end_index: None,
                        related_count: None,
                    });
                    result.push(Message::Text {
                        role: role.clone(),
                        content: truncated,
                    });
                    continue;
                }
                result.push(msg);
                continue;
            }

            if let Message::ToolResult { tool_call_id, content, .. } = &msg {
                let tokens = (content.len() + 4) / 4;
                let oversized = tokens >= oversize_threshold;
                let over_budget = running_tokens > ctx.config.compact_target();

                if oversized && !is_recent {
                    let before_tokens = tokens;
                    let truncated = truncate_text_head_tail(content, ctx.config.tool_output_max_lines);
                    let after_tokens = (truncated.len() + 4) / 4;
                    running_tokens = running_tokens.saturating_sub(before_tokens.saturating_sub(after_tokens));
                    actions.push(CompactionAction {
                        index: idx,
                        tool_name: "tool".to_string(),
                        method: CompactionMethod::OversizeCapped,
                        before_tokens,
                        after_tokens,
                        end_index: None,
                        related_count: None,
                    });
                    result.push(Message::tool_result(tool_call_id.clone(), truncated));
                    continue;
                }

                if !is_recent && !is_pinned && over_budget {
                    let before_tokens = tokens;
                    let truncated = truncate_text_head_tail(content, ctx.config.tool_output_max_lines / 2);
                    let after_tokens = (truncated.len() + 4) / 4;
                    running_tokens = running_tokens.saturating_sub(before_tokens.saturating_sub(after_tokens));
                    actions.push(CompactionAction {
                        index: idx,
                        tool_name: "tool".to_string(),
                        method: CompactionMethod::HeadTail,
                        before_tokens,
                        after_tokens,
                        end_index: None,
                        related_count: None,
                    });
                    result.push(Message::tool_result(tool_call_id.clone(), truncated));
                    continue;
                }

                result.push(msg);
                continue;
            }

            result.push(msg);
        }

        PassResult { messages: result, actions }
    }
}

fn oversize_threshold(config: &crate::core::CompactionConfig) -> usize {
    let budget_threshold = (config.budget_tokens as f64 * config.oversize_budget_ratio) as usize;
    config.oversize_abs_tokens.min(budget_threshold.max(1))
}

/// Truncate text keeping head and tail with a marker in between.
fn truncate_text_head_tail(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        return truncate_single_block(text, max_lines);
    }

    let head_count = (max_lines * 3) / 5;
    let tail_count = max_lines - head_count;
    let omitted = lines.len() - head_count - tail_count;

    let mut result = String::new();
    for line in &lines[..head_count] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!("\n[... {} lines truncated ...]\n\n", omitted));
    for (i, line) in lines[lines.len() - tail_count..].iter().enumerate() {
        result.push_str(line);
        if i < tail_count - 1 {
            result.push('\n');
        }
    }
    result
}

fn truncate_single_block(text: &str, max_lines: usize) -> String {
    let max_chars = max_lines.max(1) * 120;
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    let head_chars = max_chars * 3 / 5;
    let tail_chars = max_chars - head_chars;
    let head: String = text.chars().take(head_chars).collect();
    let tail: String = text
        .chars()
        .rev()
        .take(tail_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(
        "{}\n\n[... {} chars truncated ...]\n\n{}",
        head,
        char_count - max_chars,
        tail
    )
}
