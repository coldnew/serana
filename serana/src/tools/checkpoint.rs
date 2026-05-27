//! Checkpoint & Rewind for conversation management.

use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

/// Mark a checkpoint in the conversation for later rewind.
/// This is a marker tool — the actual checkpoint state is managed by the agent.
pub struct CheckpointTool;

#[async_trait]
impl Tool for CheckpointTool {
    fn name(&self) -> &'static str {
        "checkpoint"
    }

    fn description(&self) -> &'static str {
        "Mark the current conversation state as a checkpoint. Input: {\"label\": \"before refactor\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "description": "Label for this checkpoint"
                }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let label = input
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("checkpoint");

        // The actual checkpoint logic is in the agent layer.
        // This tool signals the agent to save state.
        Ok(json!({
            "checkpoint": true,
            "label": label,
            "message": "Checkpoint marked. Use rewind to restore this state."
        }))
    }
}

/// Rewind to a checkpoint, discarding subsequent messages.
pub struct RewindTool;

#[async_trait]
impl Tool for RewindTool {
    fn name(&self) -> &'static str {
        "rewind"
    }

    fn description(&self) -> &'static str {
        "Rewind conversation to a checkpoint, discarding exploratory context. Input: {\"label\": \"before refactor\"} or {}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "description": "Checkpoint label to rewind to (rewinds to latest if omitted)"
                }
            }
        })
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        // The actual rewind logic is in the agent layer.
        // This tool signals the agent to truncate messages.
        Ok(json!({
            "rewind": true,
            "message": "Rewind requested. Agent will truncate to the requested checkpoint."
        }))
    }
}
