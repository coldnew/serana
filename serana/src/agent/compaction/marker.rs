use crate::core::Message;

const ANCHOR_MAX_CHARS: usize = 200;
const CONCLUSION_MAX_CHARS: usize = 300;

/// Build a compaction marker that preserves key metadata from messages being removed.
pub fn build_full_marker(pre_drop_messages: &[Message], removed: usize) -> Message {
    full_marker(pre_drop_messages, &format!("{removed} messages removed"))
}

pub fn build_fallback_marker() -> Message {
    Message::system("[Context compacted: messages removed]".to_string())
}

fn full_marker(pre_drop_messages: &[Message], count_note: &str) -> Message {
    let mut out = format!("[Context compacted: {count_note}]");

    let completed = retained_user_texts(pre_drop_messages);
    if !completed.is_empty() {
        out.push_str("\n\nRetained early context contains these COMPLETED tasks (already handled, do not revisit):\n");
        for text in &completed {
            out.push_str("- ");
            out.push_str(text);
            out.push('\n');
        }
    }

    let modifications = extract_file_modifications(pre_drop_messages);
    if !modifications.is_empty() {
        out.push_str("\nFiles already modified (do not re-apply these edits):\n");
        for m in &modifications {
            out.push_str("- ");
            out.push_str(&m);
            out.push('\n');
        }
    }

    if let Some(conclusion) = latest_assistant_text(pre_drop_messages) {
        out.push_str("\nLast assistant conclusion:\n");
        out.push_str(&conclusion);
        out.push('\n');
    }

    if let Some(text) = latest_user_text(pre_drop_messages) {
        out.push_str("\nMost recent user request (verbatim):\n");
        out.push_str(&text);
    }

    Message::user(out)
}

fn retained_user_texts(messages: &[Message]) -> Vec<String> {
    let latest = latest_user_text(messages);
    let mut out = Vec::new();
    for msg in messages {
        if let Message::Text { role, content } = msg {
            if role == "user" {
                let t = content.trim();
                if marker_candidate(t) && Some(t) != latest.as_deref() {
                    out.push(trim_text(t));
                }
            }
        }
    }
    out
}

fn latest_user_text(messages: &[Message]) -> Option<String> {
    for msg in messages.iter().rev() {
        if let Message::Text { role, content } = msg {
            if role == "user" {
                let t = content.trim();
                if marker_candidate(t) {
                    return Some(trim_text(t));
                }
            }
        }
    }
    None
}

fn marker_candidate(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with("<system-reminder>")
        && !text.starts_with("[Context compacted")
}

fn trim_text(text: &str) -> String {
    if text.chars().count() > ANCHOR_MAX_CHARS {
        format!("{}…", text.chars().take(ANCHOR_MAX_CHARS).collect::<String>())
    } else {
        text.to_string()
    }
}

fn extract_file_modifications(messages: &[Message]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for msg in messages {
        if let Message::ToolResult { tool_call_id, content, .. } = msg {
            let action = "modified";
            if let Some(path) = find_tool_call_param(messages, tool_call_id, "file_path")
                .or_else(|| find_tool_call_param(messages, tool_call_id, "path"))
            {
                if seen.insert(path.clone()) {
                    result.push(format!("{path} ({action})"));
                    if result.len() >= 10 {
                        break;
                    }
                }
            }
        }
    }
    result
}

fn latest_assistant_text(messages: &[Message]) -> Option<String> {
    for msg in messages.iter().rev() {
        match msg {
            Message::Text { role, content } if role == "assistant" => {
                let t = content.trim();
                if !t.is_empty() && !is_filler(t) && !t.starts_with("[Summary]") {
                    let trimmed = if t.chars().count() > CONCLUSION_MAX_CHARS {
                        format!("{}…", t.chars().take(CONCLUSION_MAX_CHARS).collect::<String>())
                    } else {
                        t.to_string()
                    };
                    return Some(trimmed);
                }
            }
            Message::ToolCall { content: Some(c), .. } => {
                let t = c.trim();
                if !t.is_empty() && !is_filler(t) && !t.starts_with("[Summary]") {
                    let trimmed = if t.chars().count() > CONCLUSION_MAX_CHARS {
                        format!("{}…", t.chars().take(CONCLUSION_MAX_CHARS).collect::<String>())
                    } else {
                        t.to_string()
                    };
                    return Some(trimmed);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_filler(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    matches!(
        t.as_str(),
        "done" | "done." | "ok" | "ok." | "sure" | "sure."
            | "i'll fix this" | "let me check" | "let me look"
    )
}

fn find_tool_call_param(messages: &[Message], target_id: &str, param: &str) -> Option<String> {
    for msg in messages.iter().rev() {
        if let Message::ToolCall { tool_calls, .. } = msg {
            for tc in tool_calls {
                if tc.id == target_id {
                    if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                        return args.get(param).and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
        }
    }
    None
}
