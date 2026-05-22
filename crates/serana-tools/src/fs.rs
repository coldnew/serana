use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use serana_core::{Result, Tool};

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct EditFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file. Input: {\"path\": \"path/to/file\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let content = fs::read_to_string(path).await?;
        Ok(json!({"content": content, "path": path}))
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file. Input: {\"path\": \"...\", \"content\": \"...\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' field"))?;
        fs::write(path, content).await?;
        Ok(json!({"success": true, "path": path}))
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Apply hashline edits to a file. Input: {\"path\": \"...\", \"edits\": \"...\"}. \
         Edits use hashline format with line anchors like '41th|' for safe file modification."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "edits": {
                    "type": "string",
                    "description": "Hashline format edits with line anchors (e.g., '41th|line content')"
                }
            },
            "required": ["path", "edits"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        use crate::hashline::{apply_hashline, parse_hashline};

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let edits = input
            .get("edits")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'edits' field"))?;

        // Read current file content
        let content = fs::read_to_string(path).await?;

        // Parse hashline sections
        let sections = parse_hashline(edits)?;

        // Collect all ops from all sections
        let mut all_ops = Vec::new();
        for section in sections {
            all_ops.extend(section.ops);
        }

        // Apply edits
        let new_content = apply_hashline(&content, &all_ops)?;

        // Write back
        fs::write(path, &new_content).await?;

        Ok(json!({"success": true, "path": path}))
    }
}
