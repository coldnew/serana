use crate::core::{CompactionAction, CompactionMethod, Message};

use crate::agent::compressor::estimate_message_tokens;
use super::super::pass::{Pass, PassContext, PassLevel, PassResult};

/// Reclaim pass — always-on, lossless cleanup.
///
/// Clears tool results that follow a user message (they've been "used"
/// and the conversation has moved on).
pub struct Reclaim;

impl Pass for Reclaim {
    fn level(&self) -> PassLevel {
        PassLevel::Reclaim
    }

    fn should_run(&self, _ctx: &PassContext<'_>) -> bool {
        true
    }

    fn run(&self, messages: Vec<Message>, _ctx: &PassContext<'_>) -> PassResult {
        let mut has_user_after = vec![false; messages.len()];
        let mut seen_user = false;
        for i in (0..messages.len()).rev() {
            has_user_after[i] = seen_user;
            if matches!(&messages[i], Message::Text { role, .. } if role == "user") {
                seen_user = true;
            }
        }

        let mut actions = Vec::new();
        let result: Vec<Message> = messages
            .into_iter()
            .enumerate()
            .map(|(idx, msg)| {
                let should_clear = matches!(&msg, Message::ToolResult { .. }) && has_user_after[idx];

                if !should_clear {
                    return msg;
                }

                if let Message::ToolResult {
                    tool_call_id,
                    content,
                    ..
                } = msg
                {
                    let before_tokens = (content.len() + 4) / 4;
                    let marker = format!("[tool result cleared after use]");
                    let after_tokens = (marker.len() + 4) / 4;

                    actions.push(CompactionAction {
                        index: idx,
                        tool_name: "tool".to_string(),
                        method: CompactionMethod::LifecycleReclaimed,
                        before_tokens,
                        after_tokens,
                        end_index: None,
                        related_count: None,
                    });

                    Message::tool_result(tool_call_id, marker)
                } else {
                    msg
                }
            })
            .collect();

        PassResult { messages: result, actions }
    }
}
