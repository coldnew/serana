use serana_core::{AgentOutput, Message, ToolCall};

use super::ToolTurnOutput;

pub struct AgentRunState {
    messages: Vec<Message>,
    tool_calls: Vec<ToolCall>,
}

impl AgentRunState {
    pub fn new(system_prompt: String, instruction: String) -> Self {
        Self {
            messages: vec![Message::system(system_prompt), Message::user(instruction)],
            tool_calls: Vec::new(),
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn replace_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    pub fn push_assistant_tool_call(
        &mut self,
        role: String,
        content: Option<String>,
        tool_calls: Vec<serana_core::ToolCallData>,
    ) {
        self.messages.push(Message::ToolCall {
            role,
            content,
            tool_calls,
        });
    }

    pub fn apply_tool_turn(&mut self, turn: ToolTurnOutput) {
        self.messages.extend(turn.messages);
        self.tool_calls.extend(turn.tool_calls);
    }

    /// Truncate messages to the given index, preserving system prompt and user instruction.
    /// Used by checkpoint/rewind to discard exploratory context.
    pub fn truncate_to(&mut self, index: usize) {
        // Always keep at least system prompt (idx 0) and user instruction (idx 1)
        let min_keep = 2;
        let target = index.max(min_keep);
        if target < self.messages.len() {
            self.messages.truncate(target);
        }
    }

    pub fn push_system_message(&mut self, content: &str) {
        self.messages.push(Message::system(content.to_string()));
    }

    pub fn output(self, response: String) -> AgentOutput {
        AgentOutput {
            response,
            tool_calls: self.tool_calls,
            success: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn initializes_with_system_and_user_messages() {
        let state = AgentRunState::new("system".to_string(), "user".to_string());

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[0].role(), "system");
        assert_eq!(state.messages()[1].role(), "user");
    }

    #[test]
    fn applies_tool_turn_messages_and_calls() {
        let mut state = AgentRunState::new("system".to_string(), "user".to_string());
        state.apply_tool_turn(ToolTurnOutput {
            messages: vec![Message::tool_result("call_1".to_string(), "{}".to_string())],
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: json!({"path":"Cargo.toml"}),
                result: Some(json!({"ok":true})),
            }],
        });

        let output = state.output("done".to_string());
        assert_eq!(output.tool_calls.len(), 1);
        assert_eq!(output.response, "done");
    }
}
