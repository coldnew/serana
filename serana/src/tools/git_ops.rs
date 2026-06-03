//! Git integration tools for Serana.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use crate::core::{Result, Tool};

/// Show git working tree status.
pub struct GitStatusTool;

/// Show git diff.
pub struct GitDiffTool;

/// Show recent git log.
pub struct GitLogTool;

/// Create a git commit.
pub struct GitCommitTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn description(&self) -> &'static str {
        "Show git working tree status. Input: {}"
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v2"])
            .output()
            .await?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }))
    }
}

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }

    fn description(&self) -> &'static str {
        "Show git diff. Input: {\"staged\": false}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Whether to show staged diff",
                    "default": false
                }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let staged = input
            .get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut cmd = Command::new("git");
        cmd.arg("diff");
        if staged {
            cmd.arg("--staged");
        }

        let output = cmd.output().await?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }))
    }
}

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &'static str {
        "git_log"
    }

    fn description(&self) -> &'static str {
        "Show recent git log. Input: {\"limit\": 10}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Number of log entries to show",
                    "default": 10
                }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);

        let limit_str = limit.to_string();
        let output = Command::new("git")
            .args(["log", "--oneline", "-n", &limit_str])
            .output()
            .await?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }))
    }
}

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &'static str {
        "git_commit"
    }

    fn description(&self) -> &'static str {
        "Create a git commit. Input: {\"message\": \"...\", \"files\": [\"path1\"]}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "files": {
                    "type": "array",
                    "description": "Files to stage",
                    "items": {"type": "string"}
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' field"))?;

        let files: Vec<&str> = input
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        // Stage files if provided
        if !files.is_empty() {
            let add_output = Command::new("git").arg("add").args(&files).output().await?;

            if !add_output.status.success() {
                return Ok(json!({
                    "success": false,
                    "phase": "add",
                    "stdout": String::from_utf8_lossy(&add_output.stdout),
                    "stderr": String::from_utf8_lossy(&add_output.stderr),
                }));
            }
        }

        let output = Command::new("git")
            .args(["commit", "-m", message])
            .output()
            .await?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }))
    }
}
