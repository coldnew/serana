//! Mermaid diagram rendering tool.
//!
//! Generates Mermaid diagram markup and optionally renders to SVG/PNG
//! using the `mmdc` CLI (mermaid-cli).

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::{Result, Tool};

/// Tool that renders Mermaid diagrams.
///
/// Accepts Mermaid markup and either returns it as-is (for embedding)
/// or renders to SVG/PNG via the `mmdc` CLI if available.
pub struct RenderMermaidTool;

#[async_trait]
impl Tool for RenderMermaidTool {
    fn name(&self) -> &'static str {
        "render_mermaid"
    }

    fn description(&self) -> &'static str {
        "Render a Mermaid diagram. Input: {\"code\": \"graph TD; A-->B\", \"format\": \"svg\" (default) | \"png\", \"output\": \"/path/to/output.svg\" (optional)}. Returns the diagram source and optionally renders to a file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Mermaid diagram markup"
                },
                "format": {
                    "type": "string",
                    "enum": ["svg", "png"],
                    "description": "Output format (default: svg)"
                },
                "output": {
                    "type": "string",
                    "description": "Optional file path to write rendered output"
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let code = input
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'code' field"))?;

        let format = input
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("svg");

        let output_path = input.get("output").and_then(|v| v.as_str());

        // Check if mmdc (mermaid-cli) is available
        let mmdc_available = tokio::process::Command::new("which")
            .arg("mmdc")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !mmdc_available {
            // No renderer — return the source for embedding
            let mut result = json!({
                "status": "source_only",
                "code": code,
                "format": format,
                "message": "mmdc not found. Install mermaid-cli (`npm install -g @mermaid-js/mermaid-cli`) for rendering. Returning source code.",
            });
            if let Some(path) = output_path {
                // Write the mermaid source to a .mmd file
                let mmd_path = if path.ends_with(".svg") || path.ends_with(".png") {
                    path.rsplit_once('.')
                        .map(|(base, _)| format!("{}.mmd", base))
                        .unwrap_or_else(|| format!("{}.mmd", path))
                } else {
                    format!("{}.mmd", path)
                };
                tokio::fs::write(&mmd_path, code).await?;
                result["source_file"] = json!(mmd_path);
            }
            return Ok(result);
        }

        // Render with mmdc
        let temp_input = format!("/tmp/serana-mermaid-{}.mmd", uuid::Uuid::new_v4());
        let temp_output = format!("/tmp/serana-mermaid-{}.{}", uuid::Uuid::new_v4(), format);

        tokio::fs::write(&temp_input, code).await?;

        let status = tokio::process::Command::new("mmdc")
            .args([
                "-i",
                &temp_input,
                "-o",
                &temp_output,
                "-t",
                "dark",
                "-b",
                "transparent",
            ])
            .status()
            .await?;

        // Clean up temp input
        let _ = tokio::fs::remove_file(&temp_input).await;

        if !status.success() {
            let _ = tokio::fs::remove_file(&temp_output).await;
            return Err(anyhow::anyhow!(
                "mmdc rendering failed (exit code: {:?})",
                status.code()
            ));
        }

        // Read rendered output
        let rendered = tokio::fs::read(&temp_output).await?;

        // Copy to user-specified output if provided
        if let Some(path) = output_path {
            tokio::fs::write(path, &rendered).await?;
        }

        // For SVG, include the content inline
        let content = if format == "svg" {
            String::from_utf8_lossy(&rendered).to_string()
        } else {
            format!("Binary {} ({} bytes)", format, rendered.len())
        };

        // Clean up temp output (user has their copy if they specified output)
        if output_path.is_some() {
            let _ = tokio::fs::remove_file(&temp_output).await;
        }

        Ok(json!({
            "status": "rendered",
            "format": format,
            "size_bytes": rendered.len(),
            "output": output_path,
            "content": content,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mermaid_returns_source_when_no_cli() {
        let tool = RenderMermaidTool;
        let result = tool
            .execute(json!({
                "code": "graph TD; A-->B"
            }))
            .await
            .unwrap();
        // Should either render or return source
        assert!(result.get("code").is_some() || result.get("content").is_some());
    }

    #[tokio::test]
    async fn mermaid_missing_code() {
        let tool = RenderMermaidTool;
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
