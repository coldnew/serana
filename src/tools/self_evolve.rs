//! Tools for Serana to modify herself and manage her own codebase.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;

use crate::tools::Tool;
use crate::Result;
use crate::agent::{MetaCognition, ModificationKind, ModificationRecord};

/// Read Serana's own source file.
pub struct ReadSelfTool;

/// Edit Serana's own source file.
pub struct EditSelfTool;

/// Run cargo commands on Serana's codebase.
pub struct CargoTool;

/// Run git commands on Serana's codebase.
pub struct GitTool;

/// Search Serana's codebase using ripgrep pattern.
pub struct SearchCodeTool;

/// Get Serana's workspace root path.
pub struct WorkspaceRootTool;

/// Record a self-modification for learning.
pub struct RecordModificationTool;

/// Get statistics about past self-modifications.
pub struct ModificationStatsTool;

/// Reflect on a modification and add lessons learned.
pub struct ReflectModificationTool;

const SERANA_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn serana_path(relative: &str) -> PathBuf {
    PathBuf::from(SERANA_ROOT).join(relative)
}

#[async_trait]
impl Tool for ReadSelfTool {
    fn name(&self) -> &'static str {
        "read_self"
    }

    fn description(&self) -> &'static str {
        "Read a file from Serana's own source code. Input: {\"path\": \"src/agent/coding.rs\"}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let relative = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        
        // Security: prevent path traversal
        if relative.contains("..") || relative.starts_with('/') {
            anyhow::bail!("Path traversal not allowed");
        }
        
        let path = serana_path(relative);
        let content = fs::read_to_string(&path).await?;
        Ok(json!({
            "path": relative,
            "content": content,
            "absolute": path.display().to_string(),
        }))
    }
}

#[async_trait]
impl Tool for EditSelfTool {
    fn name(&self) -> &'static str {
        "edit_self"
    }

    fn description(&self) -> &'static str {
        "Edit a file in Serana's own source code. Input: {\"path\": \"src/agent/coding.rs\", \"edits\": [{\"old\": \"fn old()\", \"new\": \"fn new()\"}]} or {\"path\": \"...\", \"content\": \"full new content\"}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let relative = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        
        // Security: prevent path traversal
        if relative.contains("..") || relative.starts_with('/') {
            anyhow::bail!("Path traversal not allowed");
        }
        
        let path = serana_path(relative);
        
        if let Some(content) = input.get("content") {
            // Full file replacement
            let content = content.as_str().ok_or_else(|| anyhow::anyhow!("content must be string"))?;
            fs::write(&path, content).await?;
            return Ok(json!({
                "path": relative,
                "action": "replaced",
                "bytes_written": content.len(),
            }));
        }
        
        if let Some(edits) = input.get("edits").and_then(|v| v.as_array()) {
            // Apply multiple string replacements
            let mut content = fs::read_to_string(&path).await?;
            let mut replacements = 0;
            
            for edit in edits {
                let old = edit.get("old").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Edit missing 'old' field"))?;
                let new = edit.get("new").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Edit missing 'new' field"))?;
                
                if content.contains(old) {
                    content = content.replace(old, new);
                    replacements += 1;
                }
            }
            
            fs::write(&path, &content).await?;
            return Ok(json!({
                "path": relative,
                "action": "edited",
                "replacements": replacements,
            }));
        }
        
        anyhow::bail!("Must provide either 'content' or 'edits' field");
    }
}

#[async_trait]
impl Tool for CargoTool {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn description(&self) -> &'static str {
        "Run cargo commands on Serana's codebase. Input: {\"command\": \"test\"} or {\"command\": \"build\", \"args\": [\"--release\"]}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;
        
        let args: Vec<&str> = input
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        
        // Run cargo synchronously (simpler for self-modification verification)
        let output = Command::new("cargo")
            .current_dir(SERANA_ROOT)
            .arg(cmd)
            .args(&args)
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(json!({
            "command": format!("cargo {} {}", cmd, args.join(" ")).trim(),
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
        }))
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &'static str {
        "git"
    }

    fn description(&self) -> &'static str {
        "Run git commands on Serana's repository. Input: {\"command\": \"status\"} or {\"command\": \"commit\", \"args\": [\"-m\", \"message\"]}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;
        
        let args: Vec<&str> = input
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        
        // Safety: only allow certain git commands
        let allowed = ["status", "diff", "log", "add", "commit", "branch", "checkout", "stash", "reset", "show"];
        if !allowed.contains(&cmd) {
            anyhow::bail!("Git command '{}' not allowed for self-modification safety", cmd);
        }
        
        let output = Command::new("git")
            .current_dir(SERANA_ROOT)
            .arg(cmd)
            .args(&args)
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(json!({
            "command": format!("git {} {}", cmd, args.join(" ")).trim(),
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
        }))
    }
}

#[async_trait]
impl Tool for SearchCodeTool {
    fn name(&self) -> &'static str {
        "search_code"
    }

    fn description(&self) -> &'static str {
        "Search Serana's source code using regex pattern. Input: {\"pattern\": \"fn execute\", \"path\": \"src/agent\"}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;
        
        let search_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("src");
        
        // Security check
        if search_path.contains("..") || search_path.starts_with('/') {
            anyhow::bail!("Path traversal not allowed");
        }
        
        let full_path = serana_path(search_path);
        
        // Use ripgrep via Command (simpler than implementing regex search)
        let output = Command::new("rg")
            .current_dir(SERANA_ROOT)
            .arg("--json")
            .arg("--type")
            .arg("rust")
            .arg(pattern)
            .arg(&full_path)
            .output();
        
        let results = match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                parse_ripgrep_json(&stdout)
            }
            _ => vec![]
        };
        
        Ok(json!({
            "pattern": pattern,
            "path": search_path,
            "matches": results,
        }))
    }
}

#[async_trait]
impl Tool for WorkspaceRootTool {
    fn name(&self) -> &'static str {
        "workspace_root"
    }

    fn description(&self) -> &'static str {
        "Get the absolute path to Serana's workspace root. Input: {}"
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        Ok(json!({
            "root": SERANA_ROOT,
            "manifest": format!("{}/Cargo.toml", SERANA_ROOT),
        }))
    }
}

#[async_trait]
impl Tool for RecordModificationTool {
    fn name(&self) -> &'static str {
        "record_modification"
    }

    fn description(&self) -> &'static str {
        "Record a self-modification for learning. Input: {\"file\": \"src/agent/coding.rs\", \"kind\": \"Feature\", \"description\": \"Added X\", \"tests_passed\": true, \"commit\": \"abc123\"}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let file = input.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file' field"))?.to_string();
        let kind_str = input.get("kind").and_then(|v| v.as_str()).unwrap_or("Feature");
        let kind = match kind_str {
            "Feature" => ModificationKind::Feature,
            "BugFix" => ModificationKind::BugFix,
            "Optimization" => ModificationKind::Optimization,
            "Refactor" => ModificationKind::Refactor,
            "TestAddition" => ModificationKind::TestAddition,
            "Dependency" => ModificationKind::Dependency,
            "Config" => ModificationKind::Config,
            _ => ModificationKind::Feature,
        };
        let description = input.get("description").and_then(|v| v.as_str())
            .unwrap_or("").to_string();
        let tests_passed = input.get("tests_passed").and_then(|v| v.as_bool()).unwrap_or(false);
        let commit = input.get("commit").and_then(|v| v.as_str()).map(String::from);

        let record = ModificationRecord {
            timestamp: chrono_lite_timestamp(),
            file,
            kind,
            description,
            tests_passed,
            commit,
            lessons: vec![],
        };

        let mut meta = MetaCognition::new(PathBuf::from(SERANA_ROOT));
        meta.record(record.clone()).await?;

        Ok(json!({ "recorded": true, "timestamp": record.timestamp }))
    }
}

#[async_trait]
impl Tool for ModificationStatsTool {
    fn name(&self) -> &'static str {
        "modification_stats"
    }

    fn description(&self) -> &'static str {
        "Get statistics about past self-modifications. Input: {}"
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let meta = MetaCognition::new(PathBuf::from(SERANA_ROOT));
        let stats = meta.stats().await?;
        Ok(json!({
            "total": stats.total_modifications,
            "successful": stats.successful_modifications,
            "failed": stats.failed_modifications,
            "by_kind": stats.by_kind,
            "patterns": stats.common_patterns,
        }))
    }
}

#[async_trait]
impl Tool for ReflectModificationTool {
    fn name(&self) -> &'static str {
        "reflect_modification"
    }

    fn description(&self) -> &'static str {
        "Reflect on a modification and add lessons learned. Input: {\"file\": \"src/agent/coding.rs\", \"lessons\": [\"Test early\", \"Keep functions small\"]}"
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let file = input.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file' field"))?.to_string();
        let lessons: Vec<String> = input.get("lessons")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let mut meta = MetaCognition::new(PathBuf::from(SERANA_ROOT));
        meta.reflect(&file, lessons.clone()).await?;

        Ok(json!({ "reflected": true, "file": file, "lessons_count": lessons.len() }))
    }
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    let datetime = time_offset::from_unix_timestamp(secs as i64);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        datetime.year, datetime.month, datetime.day,
        datetime.hour, datetime.minute, datetime.second)
}

mod time_offset {
    pub struct DateTime { pub year: i32, pub month: u8, pub day: u8, pub hour: u8, pub minute: u8, pub second: u8 }
    pub fn from_unix_timestamp(ts: i64) -> DateTime {
        // Simple UTC conversion
        let days = ts / 86400;
        let secs = ts % 86400;
        let (year, month, day) = days_to_ymd(days as i32);
        DateTime { year, month, day, hour: (secs / 3600) as u8, minute: ((secs % 3600) / 60) as u8, second: (secs % 60) as u8 }
    }
    fn days_to_ymd(mut days: i32) -> (i32, u8, u8) {
        days += 719163; // Days to year 0
        let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
        let doe = days - era * 146097;
        let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365*yoe + yoe/4 - yoe/100);
        let mp = (5*doy + 2)/153;
        let d = doy - (153*mp+2)/5 + 1;
        let m = mp + (if mp < 10 { 3 } else { -9 });
        (y + (if m <= 2 { 1 } else { 0 }), m as u8, d as u8)
    }
}


fn parse_ripgrep_json(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("match"))
        .filter_map(|v| v.get("data").cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn workspace_root_returns_valid_path() {
        let result = WorkspaceRootTool.execute(json!({})).await.unwrap();
        assert!(result["root"].as_str().unwrap().ends_with("serana-new"));
    }

    #[tokio::test]
    async fn read_self_reads_own_source() {
        let result = ReadSelfTool
            .execute(json!({ "path": "src/tools/self_evolve.rs" }))
            .await
            .unwrap();
        assert!(result["content"].as_str().unwrap().contains("self_evolve"));
    }

    #[tokio::test]
    async fn edit_self_rejects_path_traversal() {
        let result = EditSelfTool
            .execute(json!({ "path": "../Cargo.toml", "content": "bad" }))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn parses_ripgrep_json_output() {
        let output = r#"{"type":"match","data":{"path":{"text":"src/main.rs"},"lines":{"text":"fn main()"}}}"#;
        let results = parse_ripgrep_json(output);
        assert_eq!(results.len(), 1);
    }
}
