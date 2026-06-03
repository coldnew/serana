use std::collections::HashSet;
use crate::core::Message;

/// Sanitize tool call / tool result pairing.
///
/// Removes orphaned tool calls (no matching result) and orphaned tool results
/// (no matching call) so the message list stays valid for providers.
pub fn sanitize_tool_pairs(messages: Vec<Message>) -> Vec<Message> {
    let mut call_ids: HashSet<String> = HashSet::new();
    let mut result_ids: HashSet<String> = HashSet::new();

    for msg in &messages {
        match msg {
            Message::ToolCall { tool_calls, .. } => {
                for tc in tool_calls {
                    call_ids.insert(tc.id.clone());
                }
            }
            Message::ToolResult { tool_call_id, .. } => {
                result_ids.insert(tool_call_id.clone());
            }
            _ => {}
        }
    }

    let orphan_calls: HashSet<String> = call_ids.difference(&result_ids).cloned().collect();
    let orphan_results: HashSet<String> = result_ids.difference(&call_ids).cloned().collect();

    if orphan_calls.is_empty() && orphan_results.is_empty() {
        return messages;
    }

    messages
        .into_iter()
        .filter_map(|msg| match msg {
            Message::ToolResult { ref tool_call_id, .. } if orphan_results.contains(tool_call_id) => {
                None
            }
            Message::ToolCall { role, content, tool_calls } => {
                let filtered: Vec<_> = tool_calls
                    .into_iter()
                    .filter(|tc| !orphan_calls.contains(&tc.id))
                    .collect();
                if !filtered.is_empty() || content.as_deref().map_or(false, |c| !c.is_empty()) {
                    Some(Message::ToolCall {
                        role,
                        content,
                        tool_calls: filtered,
                    })
                } else {
                    None
                }
            }
            other => Some(other),
        })
        .collect()
}
