//! Conflict resolution tool for git merge conflicts.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use crate::core::{Result, Tool};

/// Resolve git merge conflicts in files.
pub struct ConflictResolveTool;

#[async_trait]
impl Tool for ConflictResolveTool {
    fn name(&self) -> &'static str {
        "conflict_resolve"
    }

    fn description(&self) -> &'static str {
        "Resolve merge conflicts in a file. Input: {\"path\": \"file.rs\", \"resolution\": \"theirs\"|\"ours\"|\"base\"} \
         or {\"path\": \"file.rs\", \"resolution\": \"manual\", \"content\": \"resolved content\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file with conflicts"
                },
                "resolution": {
                    "type": "string",
                    "enum": ["theirs", "ours", "base", "manual"],
                    "description": "Resolution strategy"
                },
                "content": {
                    "type": "string",
                    "description": "Manual resolved content (when resolution=manual)"
                }
            },
            "required": ["path", "resolution"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let resolution = input
            .get("resolution")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'resolution' field"))?;

        if resolution == "manual" {
            let content = input
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'content' for manual resolution"))?;
            fs::write(path, content).await?;
            return Ok(json!({ "success": true, "path": path, "resolution": "manual" }));
        }

        let file_content = fs::read_to_string(path).await?;
        let mut resolved = String::new();
        let mut in_conflict = false;
        let mut ours_section = String::new();
        let mut theirs_section = String::new();
        let mut current_side = ""; // "ours" or "theirs"
        let mut conflicts_resolved = 0u64;

        for line in file_content.lines() {
            if line.starts_with("<<<<<<<") {
                in_conflict = true;
                ours_section.clear();
                theirs_section.clear();
                current_side = "ours";
                continue;
            }

            if in_conflict && line.starts_with("=======") {
                current_side = "theirs";
                continue;
            }

            if in_conflict && line.starts_with(">>>>>>>") {
                in_conflict = false;
                let chosen = match resolution {
                    "ours" => &ours_section,
                    "theirs" => &theirs_section,
                    "base" => {
                        // base is approximated as ours (common ancestor)
                        // Real 3-way merge requires the base version from git
                        &ours_section
                    }
                    _ => &ours_section,
                };
                resolved.push_str(chosen);
                conflicts_resolved += 1;
                continue;
            }

            if in_conflict {
                match current_side {
                    "ours" => {
                        ours_section.push_str(line);
                        ours_section.push('\n');
                    }
                    "theirs" => {
                        theirs_section.push_str(line);
                        theirs_section.push('\n');
                    }
                    _ => {}
                }
            } else {
                resolved.push_str(line);
                resolved.push('\n');
            }
        }

        fs::write(path, &resolved).await?;

        Ok(json!({
            "success": true,
            "path": path,
            "resolution": resolution,
            "conflicts_resolved": conflicts_resolved,
        }))
    }
}
