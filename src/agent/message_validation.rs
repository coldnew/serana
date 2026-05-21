//! Message alternation validation for LLM APIs.
//!
//! Ensures message sequences follow provider requirements:
//! - System -> User -> Assistant -> User -> ...
//! - Tool results can be consecutive
//! - Never two assistant messages in a row
//! - Never two user messages in a row

use crate::llm::Message;

/// Validate message alternation rules.
/// Returns Ok(()) if valid, Err with description if invalid.
pub fn validate_message_alternation(messages: &[Message]) -> Result<(), String> {
    if messages.is_empty() {
        return Ok(());
    }

    let mut expected_roles: Vec<&str> = vec!["system", "user"];

    for (i, msg) in messages.iter().enumerate() {
        let role = msg.role();

        if !expected_roles.contains(&role.as_str()) {
            return Err(format!(
                "Message {} has unexpected role '{}'. Expected one of: {:?}",
                i, role, expected_roles
            ));
        }

        expected_roles = match role.as_str() {
            "system" => vec!["user"],
            "user" => vec!["assistant"],
            "assistant" => {
                if msg.has_tool_calls() {
                    vec!["tool"]
                } else {
                    vec!["user"]
                }
            }
            "tool" => vec!["tool", "assistant"],
            _ => vec!["user", "assistant", "tool"],
        };
    }

    Ok(())
}

/// Fix message alternation by merging consecutive user messages.
pub fn fix_message_alternation(messages: &[Message]) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut fixed = Vec::with_capacity(messages.len());
    let mut current_user_content = String::new();

    for msg in messages {
        match msg {
            Message::Text { role, content } if role == "user" => {
                if !current_user_content.is_empty() {
                    current_user_content.push_str("\n\n");
                }
                current_user_content.push_str(content);
            }
            _ => {
                if !current_user_content.is_empty() {
                    fixed.push(Message::user(std::mem::take(&mut current_user_content)));
                }
                fixed.push(msg.clone());
            }
        }
    }

    if !current_user_content.is_empty() {
        fixed.push(Message::user(current_user_content));
    }

    fixed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_sequence() {
        let messages = vec![
            Message::system("You are helpful.".to_string()),
            Message::user("Hello".to_string()),
            Message::assistant("Hi there!".to_string()),
            Message::user("How are you?".to_string()),
        ];
        assert!(validate_message_alternation(&messages).is_ok());
    }

    #[test]
    fn test_invalid_consecutive_user() {
        let messages = vec![
            Message::system("You are helpful.".to_string()),
            Message::user("Hello".to_string()),
            Message::user("Are you there?".to_string()),
        ];
        assert!(validate_message_alternation(&messages).is_err());
    }

    #[test]
    fn test_fix_consecutive_user() {
        let messages = vec![
            Message::system("You are helpful.".to_string()),
            Message::user("Hello".to_string()),
            Message::user("Are you there?".to_string()),
        ];

        let fixed = fix_message_alternation(&messages);
        assert_eq!(fixed.len(), 2);
        assert!(validate_message_alternation(&fixed).is_ok());
    }
}
