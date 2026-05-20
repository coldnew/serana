use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;
use crate::tools::Tool;
use crate::Result;

pub struct ReadFileTool;
pub struct WriteFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file. Input: {\"path\": \"path/to/file\"}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path")
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

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let content = input.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' field"))?;
        fs::write(path, content).await?;
        Ok(json!({"success": true, "path": path}))
    }
}
