use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use serana_core::{Result, Tool};

/// Resolve internal URLs (skill://, memory://, agent://) or return None for regular paths.
async fn resolve_internal_url(url: &str) -> Option<Result<Value>> {
    if let Some(skill_name) = url.strip_prefix("skill://") {
        let name = skill_name.trim_end_matches('/');
        Some(resolve_skill_url(name).await)
    } else if let Some(query) = url.strip_prefix("memory://") {
        Some(resolve_memory_url(query).await)
    } else if let Some(resource) = url.strip_prefix("agent://") {
        Some(resolve_agent_url(resource).await)
    } else {
        None
    }
}

async fn resolve_skill_url(name: &str) -> Result<Value> {
    let paths = skill_search_paths();
    for base in &paths {
        let skill_path = base.join(name).join("SKILL.md");
        if skill_path.exists() {
            let content = fs::read_to_string(&skill_path).await?;
            return Ok(json!({
                "content": content,
                "path": skill_path.to_string_lossy(),
                "scheme": "skill",
                "name": name,
            }));
        }
    }
    // List available skills if not found
    let mut available = Vec::new();
    for base in &paths {
        if let Ok(mut entries) = fs::read_dir(base).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().join("SKILL.md").exists() {
                    if let Some(n) = entry.file_name().to_str() {
                        available.push(n.to_string());
                    }
                }
            }
        }
    }
    anyhow::bail!(
        "Skill '{}' not found. Available: {}",
        name,
        if available.is_empty() {
            "none".to_string()
        } else {
            available.join(", ")
        }
    )
}

fn skill_search_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".serana").join("skills"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".serana").join("skills"));
    }
    paths
}

async fn resolve_memory_url(query: &str) -> Result<Value> {
    let db_path = crate::memory::MemoryStore::db_path()?;
    if !db_path.exists() {
        return Ok(json!({
            "content": "No memory store found. Use 'retain' to store facts first.",
            "scheme": "memory",
            "query": query,
        }));
    }

    let store = crate::memory::MemoryStore::init()?;
    let results = store.search_facts("", query, 20)?;
    if results.is_empty() {
        return Ok(json!({
            "content": format!("No memories found for '{}'", query),
            "scheme": "memory",
            "query": query,
        }));
    }

    let text: String = results
        .iter()
        .map(|r| {
            let tags = r["tags"].as_str().unwrap_or("");
            let fact = r["fact"].as_str().unwrap_or("");
            if tags.is_empty() {
                fact.to_string()
            } else {
                format!("[{}] {}", tags, fact)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(json!({
        "content": text,
        "scheme": "memory",
        "query": query,
        "count": results.len(),
    }))
}

async fn resolve_agent_url(resource: &str) -> Result<Value> {
    match resource.trim_end_matches('/') {
        "context" => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let workspace_files = if let Ok(entries) = fs::read_dir(&cwd).await {
                let mut names = Vec::new();
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Some(name) = entry.file_name().to_str() {
                        names.push(name.to_string());
                    }
                }
                names.sort();
                names
            } else {
                Vec::new()
            };
            Ok(json!({
                "content": format!("Workspace: {}\nFiles: {}", cwd.display(), workspace_files.join(", ")),
                "scheme": "agent",
                "resource": resource,
            }))
        }
        "session" => {
            Ok(json!({
                "content": "Session info not yet wired to agent layer.",
                "scheme": "agent",
                "resource": resource,
            }))
        }
        _ => anyhow::bail!("Unknown agent resource: '{}'. Use 'context' or 'session'", resource),
    }
}

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct EditFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file or internal URL. Input: {\"path\": \"path/to/file\"} \
         or {\"path\": \"skill://name\"} or {\"path\": \"memory://query\"} or {\"path\": \"agent://context\"}"
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

        // Check for internal URL schemes
        if let Some(result) = resolve_internal_url(path).await {
            return result;
        }

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
