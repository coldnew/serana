use crate::core::{CompactionAction, CompactionMethod, Message};

use crate::agent::compressor::estimate_message_tokens;
use super::super::pass::{Pass, PassContext, PassLevel, PassResult};

/// Collapse pass — structural summarization of old assistant turns.
///
/// Replaces old assistant messages + their trailing tool results with
/// a short `[Summary]` line. No LLM involved.
pub struct Collapse;

impl Pass for Collapse {
    fn level(&self) -> PassLevel {
        PassLevel::Collapse
    }

    fn should_run(&self, ctx: &PassContext<'_>) -> bool {
        ctx.pressure.estimated_tokens > ctx.config.compact_trigger()
    }

    fn run(&self, messages: Vec<Message>, ctx: &PassContext<'_>) -> PassResult {
        let len = messages.len();
        if len <= ctx.config.keep_recent {
            return PassResult { messages, actions: vec![] };
        }

        let boundary = len - ctx.config.keep_recent;
        let compact_target = ctx.config.compact_target();
        let mut result = Vec::new();
        let mut actions = Vec::new();
        let mut running_tokens = ctx.pressure.estimated_tokens;

        let mut i = 0;
        while i < boundary {
            if running_tokens <= compact_target {
                while i < boundary {
                    result.push(messages[i].clone());
                    i += 1;
                }
                break;
            }

            let msg = &messages[i];
            match msg {
                Message::ToolCall { content, tool_calls, .. } => {
                    let turn_start = i;
                    let before_assistant = estimate_token_count(msg);

                    let tool_names: Vec<_> = tool_calls
                        .iter()
                        .map(|tc| tc.function.name.clone())
                        .collect();

                    let text_part = content.as_deref().unwrap_or("");

                    let summary = if !tool_names.is_empty() {
                        let tools_part = if tool_names.len() <= 3 {
                            tool_names.join(", ")
                        } else {
                            format!("[Assistant used {} tool(s)]", tool_names.len())
                        };
                        if !text_part.is_empty() && text_part.len() <= 200 {
                            format!("[Summary] {} — \"{}\"", tools_part, text_part)
                        } else {
                            format!("[Summary] {}", tools_part)
                        }
                    } else if !text_part.is_empty() && text_part.len() <= 200 {
                        format!("[Summary] {}", text_part)
                    } else {
                        "[Summary] [Assistant response]".to_string()
                    };

                    let after_assistant = estimate_token_count(&Message::Text {
                        role: "assistant".to_string(),
                        content: summary.clone(),
                    });

                    let mut peek = i + 1;
                    let mut tool_result_count: usize = 0;
                    let mut tool_result_tokens: usize = 0;
                    while peek < boundary {
                        if matches!(&messages[peek], Message::ToolResult { .. }) {
                            tool_result_tokens += estimate_token_count(&messages[peek]);
                            tool_result_count += 1;
                            peek += 1;
                        } else {
                            break;
                        }
                    }

                    let total_before = before_assistant + tool_result_tokens;
                    if after_assistant < total_before {
                        running_tokens = running_tokens.saturating_sub(total_before.saturating_sub(after_assistant));
                        result.push(Message::Text {
                            role: "assistant".to_string(),
                            content: summary,
                        });
                        i = peek;

                        actions.push(CompactionAction {
                            index: turn_start,
                            tool_name: "assistant".to_string(),
                            method: CompactionMethod::TurnCollapsed,
                            before_tokens: total_before,
                            after_tokens: after_assistant,
                            end_index: None,
                            related_count: Some(tool_result_count),
                        });
                        continue;
                    }

                    result.push(msg.clone());
                    i += 1;
                    while i < boundary {
                        if matches!(&messages[i], Message::ToolResult { .. }) {
                            result.push(messages[i].clone());
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    continue;
                }
                Message::Text { role, .. } if role == "assistant" => {
                    let turn_start = i;
                    let before_assistant = estimate_token_count(msg);
                    let text = match msg {
                        Message::Text { content, .. } => content.clone(),
                        _ => String::new(),
                    };

                    let summary = if text.len() <= 200 {
                        format!("[Summary] {}", text)
                    } else {
                        "[Summary] [Assistant response]".to_string()
                    };
                    let after_assistant = estimate_token_count(&Message::Text {
                        role: "assistant".to_string(),
                        content: summary.clone(),
                    });

                    let mut peek = i + 1;
                    let mut tool_result_count: usize = 0;
                    let mut tool_result_tokens: usize = 0;
                    while peek < boundary {
                        if matches!(&messages[peek], Message::ToolResult { .. }) {
                            tool_result_tokens += estimate_token_count(&messages[peek]);
                            tool_result_count += 1;
                            peek += 1;
                        } else {
                            break;
                        }
                    }

                    let total_before = before_assistant + tool_result_tokens;
                    if after_assistant < total_before {
                        running_tokens = running_tokens.saturating_sub(total_before.saturating_sub(after_assistant));
                        result.push(Message::Text {
                            role: "assistant".to_string(),
                            content: summary,
                        });
                        i = peek;

                        actions.push(CompactionAction {
                            index: turn_start,
                            tool_name: "assistant".to_string(),
                            method: CompactionMethod::TurnCollapsed,
                            before_tokens: total_before,
                            after_tokens: after_assistant,
                            end_index: None,
                            related_count: Some(tool_result_count),
                        });
                        continue;
                    }

                    result.push(msg.clone());
                    i += 1;
                    continue;
                }
                _ => {
                    result.push(msg.clone());
                }
            }
            i += 1;
        }

        result.extend_from_slice(&messages[boundary..]);

        PassResult { messages: result, actions }
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
