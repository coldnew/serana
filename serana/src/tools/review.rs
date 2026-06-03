//! Code review tool that uses LLM to analyze diffs.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::{Result, Tool};

/// Code review tool - sends a diff to the LLM for structured review.
pub struct CodeReviewTool;

#[async_trait]
impl Tool for CodeReviewTool {
    fn name(&self) -> &'static str {
        "code_review"
    }

    fn description(&self) -> &'static str {
        "Review code for issues. Input: {\"diff\": \"...\", \"context\": \"optional context\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "diff": {
                    "type": "string",
                    "description": "The diff or code to review"
                },
                "context": {
                    "type": "string",
                    "description": "Additional context for the review"
                }
            },
            "required": ["diff"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let diff = input
            .get("diff")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'diff' field"))?;
        let context = input.get("context").and_then(|v| v.as_str()).unwrap_or("");

        // This tool returns a structured prompt that the agent should
        // send to the LLM. The actual LLM call happens in the agent layer.
        Ok(json!({
            "review_request": true,
            "diff": diff,
            "context": context,
            "instructions": "Analyze this code change for bugs, security issues, performance problems, and style issues. For each issue found, provide: severity (P0-P3), file, line, description, and suggested fix."
        }))
    }
}
