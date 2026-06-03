use crate::core::{CompactionAction, CompactionMethod, Message};

use crate::agent::compressor::estimate_message_tokens;
use super::super::marker;
use super::super::pass::{Pass, PassContext, PassLevel, PassResult};
use super::super::pressure::Pressure;

/// Evict pass — remove stale message spans when smaller passes cannot fit context.
pub struct Evict;

impl Pass for Evict {
    fn level(&self) -> PassLevel {
        PassLevel::Evict
    }

    fn should_run(&self, ctx: &PassContext<'_>) -> bool {
        ctx.pressure.estimated_tokens > ctx.config.budget_tokens
            || (ctx.config.max_messages > 0 && ctx.pressure.message_count > ctx.config.max_messages)
    }

    fn run(&self, messages: Vec<Message>, ctx: &PassContext<'_>) -> PassResult {
        let len = messages.len();

        let over_tokens = ctx.pressure.estimated_tokens > ctx.config.budget_tokens;
        let over_messages = ctx.config.max_messages > 0 && len > ctx.config.max_messages;

        if over_tokens {
            if let Some(plan) = eviction_plan(&messages, ctx, ctx.pressure.estimated_tokens) {
                return apply_plan(messages, plan);
            }
        }

        if over_messages {
            let pct_target = ctx.config.max_messages * ctx.config.message_limit_target_pct as usize / 100;
            let minimum = ctx.config.keep_first + ctx.config.keep_recent + 1;
            let target_len = pct_target.max(minimum).min(ctx.config.max_messages);

            if len <= target_len {
                return PassResult { messages, actions: vec![] };
            }

            let to_remove = len - target_len;
            let plan = EvictionPlan {
                start: ctx.config.keep_first,
                end: ctx.config.keep_first + to_remove,
                marker: true,
            };
            return apply_plan(messages, plan);
        }

        PassResult { messages, actions: vec![] }
    }
}

struct EvictionPlan {
    start: usize,
    end: usize,
    marker: bool,
}

fn eviction_plan(messages: &[Message], ctx: &PassContext<'_>, current_tokens: usize) -> Option<EvictionPlan> {
    if current_tokens <= ctx.config.budget_tokens {
        return None;
    }

    let len = messages.len();
    let keep_first = ctx.config.keep_first;
    let keep_recent = ctx.config.keep_recent;
    let end_limit = len.saturating_sub(keep_recent);
    if end_limit <= keep_first {
        return None;
    }

    for start in keep_first..end_limit {
        let mut freed: usize = 0;
        for end in (start + 1)..=end_limit {
            freed += estimate_token_count(&messages[end - 1]);
            let after = current_tokens.saturating_sub(freed);
            if after <= ctx.config.budget_tokens {
                let marker_token_estimate = 50;
                let need_marker = after + marker_token_estimate <= ctx.config.budget_tokens;
                return Some(EvictionPlan {
                    start,
                    end,
                    marker: need_marker,
                });
            }
        }
    }

    None
}

fn apply_plan(messages: Vec<Message>, plan: EvictionPlan) -> PassResult {
    let before_tokens = estimate_message_tokens(&messages);
    let removed = messages[plan.start..plan.end].len();
    let before_count = messages.len();

    let mut result: Vec<Message> = messages
        .into_iter()
        .enumerate()
        .filter_map(|(i, msg)| {
            if i >= plan.start && i < plan.end {
                None
            } else {
                Some(msg)
            }
        })
        .collect();

    if plan.marker {
        let last = result.last().cloned().unwrap_or_else(|| Message::system("".to_string()));
        let marker_msg = marker::build_full_marker(
            &[last.clone()],
            removed,
        );
        result.push(marker_msg);
    }

    let after_tokens = estimate_message_tokens(&result);

    PassResult {
        messages: result,
        actions: vec![CompactionAction {
            index: plan.start,
            tool_name: "messages".to_string(),
            method: CompactionMethod::MessagesEvicted,
            before_tokens,
            after_tokens,
            end_index: Some(plan.end),
            related_count: Some(removed),
        }],
    }
}

fn estimate_token_count(msg: &Message) -> usize {
    match msg {
        Message::Text { content, .. } => (content.len() + 4) / 4,
        Message::ToolCall { content, tool_calls, .. } => {
            let text_len = content.as_deref().map(|c| c.len()).unwrap_or(0);
            let calls_len: usize = tool_calls
                .iter()
                .map(|tc| tc.function.name.len() + tc.function.arguments.len() + 20)
                .sum();
            (text_len + calls_len + 4) / 4
        }
        Message::ToolResult { content, .. } => (content.len() + 4) / 4,
    }
}
