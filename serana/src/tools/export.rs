//! Export tool — exports conversation/session data to various formats.
//!
//! Supports Markdown, JSON, and HTML export of session history.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::{Result, Tool};

/// Tool that exports conversation data to files.
pub struct ExportTool;

#[async_trait]
impl Tool for ExportTool {
    fn name(&self) -> &'static str {
        "export"
    }

    fn description(&self) -> &'static str {
        "Export conversation or session data to a file. Input: {\"format\": \"markdown\" | \"json\", \"path\": \"/path/to/export.md\", \"messages\": [{\"role\": \"user\", \"content\": \"...\"}...] (optional)}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "description": "Export format (default: markdown)"
                },
                "path": {
                    "type": "string",
                    "description": "Output file path"
                },
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": { "type": "string" },
                            "content": { "type": "string" }
                        }
                    },
                    "description": "Messages to export (if empty, exports current session)"
                },
                "title": {
                    "type": "string",
                    "description": "Optional title for the export"
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

        let format = input
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Serana Session Export");

        let messages = input
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let content = match format {
            "json" => serde_json::to_string_pretty(&json!({
                "title": title,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "messages": messages,
            }))?,
            "markdown" | _ => {
                let mut md = String::new();
                md.push_str(&format!("# {}\n\n", title));
                md.push_str(&format!(
                    "*Exported: {}*\n\n---\n\n",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
                ));
                for msg in &messages {
                    let role = msg
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let icon = match role {
                        "user" => "**You**",
                        "assistant" | "agent" => "**Agent**",
                        "system" => "**System**",
                        "tool" => "**Tool**",
                        _ => role,
                    };
                    md.push_str(&format!("{}:\n{}\n\n", icon, content));
                }
                md
            }
        };

        tokio::fs::write(path, &content).await?;

        Ok(json!({
            "status": "exported",
            "path": path,
            "format": format,
            "message_count": messages.len(),
            "size_bytes": content.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_support::tempdir;

    #[tokio::test]
    async fn export_markdown() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("export.md");
        let tool = ExportTool;
        let result = tool
            .execute(json!({
                "path": out.to_string_lossy(),
                "format": "markdown",
                "title": "Test Export",
                "messages": [
                    {"role": "user", "content": "Hello"},
                    {"role": "assistant", "content": "Hi there!"}
                ]
            }))
            .await
            .unwrap();
        assert_eq!(result["status"], "exported");
        assert_eq!(result["message_count"], 2);
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("Test Export"));
        assert!(content.contains("Hello"));
        assert!(content.contains("Hi there!"));
    }

    #[tokio::test]
    async fn export_json() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("export.json");
        let tool = ExportTool;
        let result = tool
            .execute(json!({
                "path": out.to_string_lossy(),
                "format": "json",
                "messages": [{"role": "user", "content": "test"}]
            }))
            .await
            .unwrap();
        assert_eq!(result["format"], "json");
        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("messages").is_some());
    }
}
