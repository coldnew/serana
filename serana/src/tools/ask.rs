//! Ask tool — prompts the user for input or confirmation.
//!
//! Signals the TUI that the agent is waiting for user input,
//! allowing interactive workflows (e.g., "Should I proceed?", "Pick an option").

use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

/// Tool that prompts the user for input via the TUI.
///
/// The agent uses this when it needs user confirmation, a choice,
/// or additional information before proceeding.
pub struct AskTool;

#[async_trait]
impl Tool for AskTool {
    fn name(&self) -> &'static str {
        "ask"
    }

    fn description(&self) -> &'static str {
        "Prompt the user for input, confirmation, or a choice. Use when you need user guidance before proceeding. Input: {\"prompt\": \"Your question\", \"options\": [\"option1\", \"option2\"] (optional), \"default\": \"option1\" (optional)}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The question or prompt to display to the user"
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of choices for the user to pick from"
                },
                "default": {
                    "type": "string",
                    "description": "Optional default answer if user provides no input"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' field"))?;

        let options: Option<Vec<String>> = input
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        let default = input
            .get("default")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Build the response message for the TUI/agent loop.
        // The TUI should intercept this and display the prompt to the user,
        // then feed the user's response back as the next user message.
        let mut result = json!({
            "action": "ask_user",
            "prompt": prompt,
            "waiting": true,
        });

        if let Some(opts) = options {
            result["options"] = json!(opts);
        }
        if let Some(def) = default {
            result["default"] = json!(def);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ask_basic_prompt() {
        let tool = AskTool;
        let result = tool
            .execute(json!({"prompt": "Should I continue?"}))
            .await
            .unwrap();
        assert_eq!(result["action"], "ask_user");
        assert_eq!(result["prompt"], "Should I continue?");
        assert_eq!(result["waiting"], true);
    }

    #[tokio::test]
    async fn ask_with_options() {
        let tool = AskTool;
        let result = tool
            .execute(json!({
                "prompt": "Pick a color",
                "options": ["red", "green", "blue"],
                "default": "green"
            }))
            .await
            .unwrap();
        assert_eq!(result["options"], json!(["red", "green", "blue"]));
        assert_eq!(result["default"], "green");
    }

    #[tokio::test]
    async fn ask_missing_prompt() {
        let tool = AskTool;
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
