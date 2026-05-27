//! Clipboard tool for copy/paste text.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

use serana_core::{Result, Tool};

/// Copy text to the system clipboard.
pub struct ClipboardCopyTool;

#[async_trait]
impl Tool for ClipboardCopyTool {
    fn name(&self) -> &'static str {
        "clipboard_copy"
    }

    fn description(&self) -> &'static str {
        "Copy text to the system clipboard. Input: {\"text\": \"...\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to copy to clipboard"
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'text' field"))?;

        // Try xclip (Linux), pbcopy (macOS), clip.exe (WSL/Windows)
        let result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()?;
                return Ok(json!({ "success": true, "copied": text.len() }));
            }
            Err(_) => {}
        }

        // Fallback to pbcopy
        let result = Command::new("pbcopy").spawn();
        match result {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()?;
                return Ok(json!({ "success": true, "copied": text.len() }));
            }
            Err(_) => {}
        }

        Err(anyhow::anyhow!(
            "No clipboard tool found (tried xclip, pbcopy)"
        ))
    }
}

/// Paste text from the system clipboard.
pub struct ClipboardPasteTool;

#[async_trait]
impl Tool for ClipboardPasteTool {
    fn name(&self) -> &'static str {
        "clipboard_paste"
    }

    fn description(&self) -> &'static str {
        "Paste text from the system clipboard. Input: {}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        // Try xclip (Linux)
        let result = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                return Ok(json!({ "text": text, "length": text.len() }));
            }
            _ => {}
        }

        // Fallback to pbpaste
        let result = Command::new("pbpaste").output();
        match result {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                return Ok(json!({ "text": text, "length": text.len() }));
            }
            _ => {}
        }

        Err(anyhow::anyhow!(
            "No clipboard tool found (tried xclip, pbpaste)"
        ))
    }
}
