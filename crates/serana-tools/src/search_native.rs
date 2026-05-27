//! In-process file search using glob patterns.
//!
//! Replaces subprocess `find` / `fd` with native Rust glob matching.
//! Respects .gitignore via the `ignore` crate.

use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

/// Find files using glob patterns, with .gitignore support.
pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &'static str {
        "find"
    }

    fn description(&self) -> &'static str {
        "Find files using glob patterns. Input: {\"pattern\": \"**/*.rs\", \"limit\": 100}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. **/*.rs, src/**/*.ts)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 100)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;

        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        // Use ignore crate's WalkBuilder for gitignore-aware walking
        let cwd = std::env::current_dir()?;
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern: {}", e))?;

        let mut results = Vec::new();

        for entry in ignore::WalkBuilder::new(&cwd).hidden(false).build() {
            if results.len() >= limit {
                break;
            }
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let relative = path.strip_prefix(&cwd).unwrap_or(path);
                    let relative_str = relative.to_string_lossy();
                    if glob_pattern.matches(&relative_str) {
                        results.push(json!({
                            "path": relative_str.to_string(),
                            "is_dir": path.is_dir(),
                        }));
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(json!({
            "pattern": pattern,
            "count": results.len(),
            "files": results,
        }))
    }
}

/// Search file contents using regex patterns.
pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "Search file contents using regex. Input: {\"pattern\": \"fn main\", \"paths\": [\"src/\"], \"limit\": 50}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "paths": {
                    "type": "array",
                    "description": "Paths to search in (default: current directory)",
                    "items": { "type": "string" }
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of matches (default: 50)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;

        let paths: Vec<String> = input
            .get("paths")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec![".".to_string()]);

        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let re = regex::Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;

        let cwd = std::env::current_dir()?;
        let mut results = Vec::new();

        for search_path in &paths {
            if results.len() >= limit {
                break;
            }

            let base = cwd.join(search_path);

            for entry in ignore::WalkBuilder::new(&base).hidden(false).build() {
                if results.len() >= limit {
                    break;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // Only search text-like files
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext,
                        "png"
                            | "jpg"
                            | "jpeg"
                            | "gif"
                            | "webp"
                            | "exe"
                            | "bin"
                            | "so"
                            | "dylib"
                            | "dll"
                            | "pdf"
                            | "zip"
                            | "tar"
                            | "gz"
                    ) {
                        continue;
                    }
                }

                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue, // Skip binary files
                };

                let relative = path.strip_prefix(&cwd).unwrap_or(path);

                for (line_num, line) in content.lines().enumerate() {
                    if results.len() >= limit {
                        break;
                    }
                    if re.is_match(line) {
                        results.push(json!({
                            "file": relative.to_string_lossy(),
                            "line": line_num + 1,
                            "content": line.trim(),
                        }));
                    }
                }
            }
        }

        Ok(json!({
            "pattern": pattern,
            "count": results.len(),
            "matches": results,
        }))
    }
}
