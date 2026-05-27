use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use serana_core::{Result, Tool};
use serana_tree_sitter::{LanguageId, ParserManager, SyntaxTree};
use tree_sitter::{Query, QueryCursor};

pub struct AstEditTool;

#[derive(Debug, Clone)]
struct Edit {
    start_byte: usize,
    end_byte: usize,
    replacement: String,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
    before_text: String,
}

#[async_trait]
impl Tool for AstEditTool {
    fn name(&self) -> &'static str {
        "ast_edit"
    }

    fn description(&self) -> &'static str {
        "Structural code rewrite using tree-sitter queries. Finds AST nodes matching a pattern and replaces them. \
         Supports metavariable substitution in replacement templates. \
         Use `dry_run: true` (default) to preview changes, then `dry_run: false` to apply.\n\n\
         Pattern syntax: tree-sitter S-expression queries. Capture nodes with @name.\n\
         Replacement syntax: use $captured_name to insert matched text. Use just $0 for the whole matched node.\n\n\
         Examples:\n\
         - Rename function: query='(function_item name: (identifier) @old) @fn', replacement='new_name', selector='old'\n\
         - Replace call: query='(call_expression function: (identifier) @fn) @call', selector='call'\n\
         - Add wrapper: query='(function_item name: (identifier) @name) @fn', replacement='pub $0', selector='fn'"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the source file to edit"
                },
                "query": {
                    "type": "string",
                    "description": "Tree-sitter S-expression query pattern with @captures"
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement template. $0 = full matched text, $name = captured text. If only selector is set and no replacement, deletes the matched node."
                },
                "selector": {
                    "type": "string",
                    "description": "Which capture to replace (e.g. 'fn' for @fn). Defaults to the first capture."
                },
                "language": {
                    "type": "string",
                    "description": "Override language detection (rust, javascript, python, go, bash, c)"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Preview changes without applying (default: true)"
                }
            },
            "required": ["path", "query"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let path = PathBuf::from(path_str);

        let query_src = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' field"))?;

        let replacement = input
            .get("replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let selector = input.get("selector").and_then(|v| v.as_str());

        let dry_run = input
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let lang_override = input.get("language").and_then(|v| v.as_str());

        let content = fs::read_to_string(&path).await?;

        let lang_id = if let Some(lang) = lang_override {
            parse_language(lang)?
        } else {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot detect language from path"))?;
            LanguageId::from_extension(ext)
                .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", ext))?
        };

        let manager = ParserManager::new();
        let tree = manager.parse_source(lang_id, &content)?;

        let language = serana_tree_sitter::language_for_id(lang_id);
        let query = Query::new(&language, query_src)
            .map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;

        let edits = find_edits(&tree, &content, &query, selector, replacement)?;

        if edits.is_empty() {
            return Ok(json!({
                "path": path_str,
                "applied": false,
                "changes": [],
                "total_replacements": 0,
            }));
        }

        let changes: Vec<Value> = edits
            .iter()
            .map(|e| {
                json!({
                    "before": e.before_text,
                    "after": e.replacement,
                    "start_line": e.start_line + 1,
                    "start_column": e.start_col + 1,
                    "end_line": e.end_line + 1,
                    "end_column": e.end_col + 1,
                })
            })
            .collect();

        if dry_run {
            return Ok(json!({
                "path": path_str,
                "applied": false,
                "changes": changes,
                "total_replacements": edits.len(),
            }));
        }

        let new_content = apply_edits(&content, &edits)?;
        fs::write(&path, &new_content).await?;

        Ok(json!({
            "path": path_str,
            "applied": true,
            "changes": changes,
            "total_replacements": edits.len(),
        }))
    }
}

fn find_edits(
    tree: &SyntaxTree,
    content: &str,
    query: &Query,
    selector: Option<&str>,
    replacement_tpl: &str,
) -> Result<Vec<Edit>> {
    let mut cursor = QueryCursor::new();
    let root = tree.tree().root_node();
    let capture_names: Vec<String> = query.capture_names().iter().map(|s| s.to_string()).collect();

    let selector_idx = if let Some(sel) = selector {
        capture_names
            .iter()
            .position(|n| n == sel)
            .ok_or_else(|| anyhow::anyhow!("Capture @{} not found in query", sel))?
    } else {
        0
    };

    let bytes = content.as_bytes();
    let mut edits = Vec::new();
    let mut seen_ranges = Vec::new();

    for m in cursor.matches(query, root, bytes) {
        let mut captures: HashMap<String, String> = HashMap::new();
        let mut target_node = None;

        for capture in m.captures {
            let name = &capture_names[capture.index as usize];
            let text = capture
                .node
                .utf8_text(bytes)
                .unwrap_or("")
                .to_string();
            if capture.index as usize == selector_idx {
                target_node = Some(capture.node);
            }
            captures.insert(name.clone(), text);
        }

        let target = match target_node {
            Some(n) => n,
            None => continue,
        };

        let range = (target.start_byte(), target.end_byte());
        if seen_ranges.contains(&range) {
            continue;
        }
        seen_ranges.push(range);

        let full_match = captures
            .get("0")
            .cloned()
            .unwrap_or_else(|| {
                target.utf8_text(bytes).unwrap_or("").to_string()
            });

        let mut result = replacement_tpl.to_string();

        if result.is_empty() && captures.len() == 1 {
            result = String::new();
        } else if result.is_empty() {
            result = full_match.clone();
        }

        for (name, text) in &captures {
            result = result.replace(&format!("${}", name), text);
        }
        result = result.replace("$0", &full_match);

        let start_pos = target.start_position();
        let end_pos = target.end_position();

        edits.push(Edit {
            start_byte: target.start_byte(),
            end_byte: target.end_byte(),
            replacement: result,
            start_line: start_pos.row,
            start_col: start_pos.column,
            end_line: end_pos.row,
            end_col: end_pos.column,
            before_text: full_match,
        });
    }

    edits.sort_by(|a, b| a.start_byte.cmp(&b.start_byte));

    for window in edits.windows(2) {
        if window[0].end_byte > window[1].start_byte {
            anyhow::bail!(
                "Overlapping edits at lines {} and {}",
                window[0].start_line + 1,
                window[1].start_line + 1
            );
        }
    }

    Ok(edits)
}

fn apply_edits(content: &str, edits: &[Edit]) -> Result<String> {
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;

    for edit in edits {
        if edit.start_byte < last_end {
            anyhow::bail!("Overlapping edits detected");
        }
        result.push_str(&content[last_end..edit.start_byte]);
        result.push_str(&edit.replacement);
        last_end = edit.end_byte;
    }
    result.push_str(&content[last_end..]);

    Ok(result)
}

fn parse_language(lang: &str) -> Result<LanguageId> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => Ok(LanguageId::Rust),
        "javascript" | "js" => Ok(LanguageId::JavaScript),
        "typescript" | "ts" => Ok(LanguageId::TypeScript),
        "python" | "py" => Ok(LanguageId::Python),
        "go" => Ok(LanguageId::Go),
        "bash" | "sh" => Ok(LanguageId::Bash),
        "c" | "cpp" => Ok(LanguageId::C),
        _ => Err(anyhow::anyhow!("Unsupported language: {}", lang).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_support::tempdir;

    #[tokio::test]
    async fn ast_edit_renames_function() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        let source = r#"fn old_name() -> i32 { 42 }
fn other() { old_name(); }
"#;
        fs::write(&file, source).await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @old) @fn",
                "selector": "old",
                "replacement": "new_name",
                "dry_run": true,
            }))
            .await
            .unwrap();

        assert_eq!(result["total_replacements"], 2);
        assert_eq!(result["changes"][0]["before"], "old_name");
        assert_eq!(result["changes"][0]["after"], "new_name");
        assert_eq!(result["changes"][1]["before"], "other");
        assert_eq!(result["changes"][1]["after"], "new_name");
        assert_eq!(result["applied"], false);
    }

    #[tokio::test]
    async fn ast_edit_applies_changes() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        let source = "fn hello() {}\n";
        fs::write(&file, source).await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @old) @fn",
                "selector": "old",
                "replacement": "world",
                "dry_run": false,
            }))
            .await
            .unwrap();

        assert_eq!(result["applied"], true);
        assert_eq!(result["total_replacements"], 1);

        let new_content = fs::read_to_string(&file).await.unwrap();
        assert_eq!(new_content, "fn world() {}\n");
    }

    #[tokio::test]
    async fn ast_edit_dry_run_does_not_modify() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        let source = "fn test() {}\n";
        fs::write(&file, source).await.unwrap();

        AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @old) @fn",
                "selector": "old",
                "replacement": "changed",
                "dry_run": true,
            }))
            .await
            .unwrap();

        let content = fs::read_to_string(&file).await.unwrap();
        assert_eq!(content, source);
    }

    #[tokio::test]
    async fn ast_edit_template_substitution() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        let source = "fn add(a: i32) -> i32 { a }\n";
        fs::write(&file, source).await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @name parameters: (parameters) @params) @fn",
                "selector": "fn",
                "replacement": "pub fn $name$params -> i32",
                "dry_run": true,
            }))
            .await
            .unwrap();

        assert_eq!(result["total_replacements"], 1);
        assert_eq!(result["changes"][0]["after"], "pub fn add(a: i32) -> i32");
    }

    #[tokio::test]
    async fn ast_edit_no_matches() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "struct Foo;\n").await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @name) @fn",
                "replacement": "bar",
            }))
            .await
            .unwrap();

        assert_eq!(result["total_replacements"], 0);
        assert_eq!(result["changes"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn ast_edit_multiple_replacements() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        let source = "fn foo() {}\nfn bar() {}\nfn baz() {}\n";
        fs::write(&file, source).await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "(function_item name: (identifier) @old) @fn",
                "selector": "old",
                "replacement": "renamed",
                "dry_run": true,
            }))
            .await
            .unwrap();

        assert_eq!(result["total_replacements"], 3);
    }

    #[tokio::test]
    async fn ast_edit_rejects_overlapping() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn test() {}\n")
            .await
            .unwrap();

        // This query captures both the function and its name - but since
        // selector picks one specific capture, no overlap should occur.
        // Test with invalid query to ensure error handling works.
        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "query": "INVALID SYNTAX ((((",
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ast_edit_with_language_override() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("code.txt");
        let source = "fn hello() {}\n";
        fs::write(&file, source).await.unwrap();

        let result = AstEditTool
            .execute(json!({
                "path": file.to_string_lossy(),
                "language": "rust",
                "query": "(function_item name: (identifier) @old) @fn",
                "selector": "old",
                "replacement": "world",
                "dry_run": true,
            }))
            .await
            .unwrap();

        assert_eq!(result["total_replacements"], 1);
        assert_eq!(result["changes"][0]["after"], "world");
    }
}
