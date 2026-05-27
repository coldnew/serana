use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use serana_core::{Result, Tool};
use serana_tree_sitter::{ParserManager, SyntaxTree};
use tree_sitter::Node;

pub struct AstGrepTool;

#[async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &'static str {
        "ast_grep"
    }

    fn description(&self) -> &'static str {
        "Search for AST patterns in source code using tree-sitter. Returns matching nodes with their location. Input: {\"path\": \"src/main.rs\", \"pattern\": \"fn $NAME\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the source file to search"
                },
                "pattern": {
                    "type": "string",
                    "description": "Tree-sitter pattern to match (e.g. 'fn $NAME', 'let $VAR = $EXPR')"
                }
            },
            "required": ["path", "pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;

        let content = fs::read_to_string(path).await?;
        let manager = ParserManager::new();
        let tree = manager.parse_file(std::path::Path::new(path), &content)?;

        let matches = find_pattern_matches(&tree, &content, pattern, path);
        Ok(json!({
            "path": path,
            "pattern": pattern,
            "matches": matches,
        }))
    }
}

fn find_pattern_matches(
    tree: &SyntaxTree,
    content: &str,
    pattern: &str,
    file_path: &str,
) -> Vec<Value> {
    let mut results = Vec::new();
    let root = tree.tree().root_node();

    let lines: Vec<&str> = content.lines().collect();

    match pattern {
        p if p.starts_with("fn ") || p == "fn $NAME" => {
            find_nodes_by_kind(root, "function_item", &lines, file_path, &mut results);
            find_nodes_by_kind(root, "function_definition", &lines, file_path, &mut results);
        }
        p if p.starts_with("let ") || p == "let $VAR = $EXPR" => {
            find_nodes_by_kind(root, "let_declaration", &lines, file_path, &mut results);
        }
        p if p.starts_with("struct ") || p == "struct $NAME" => {
            find_nodes_by_kind(root, "struct_item", &lines, file_path, &mut results);
        }
        p if p.starts_with("impl ") || p == "impl $NAME" => {
            find_nodes_by_kind(root, "impl_item", &lines, file_path, &mut results);
        }
        p if p.starts_with("use ") || p == "use $PATH" => {
            find_nodes_by_kind(root, "use_declaration", &lines, file_path, &mut results);
        }
        p if p.starts_with("mod ") || p == "mod $NAME" => {
            find_nodes_by_kind(root, "mod_item", &lines, file_path, &mut results);
        }
        p if p.starts_with("trait ") || p == "trait $NAME" => {
            find_nodes_by_kind(root, "trait_item", &lines, file_path, &mut results);
        }
        p if p.starts_with("enum ") || p == "enum $NAME" => {
            find_nodes_by_kind(root, "enum_item", &lines, file_path, &mut results);
        }
        p if p == "if $COND" || p.starts_with("if ") => {
            find_nodes_by_kind(root, "if_expression", &lines, file_path, &mut results);
        }
        p if p == "match $EXPR" || p.starts_with("match ") => {
            find_nodes_by_kind(root, "match_expression", &lines, file_path, &mut results);
        }
        p if p == "for $VAR in $ITER" || p.starts_with("for ") => {
            find_nodes_by_kind(root, "for_expression", &lines, file_path, &mut results);
        }
        p if p == "while $COND" || p.starts_with("while ") => {
            find_nodes_by_kind(root, "while_expression", &lines, file_path, &mut results);
        }
        _ => {
            find_text_in_nodes(root, pattern, &lines, file_path, &mut results);
        }
    }

    results
}

fn find_nodes_by_kind(
    node: Node,
    kind: &str,
    lines: &[&str],
    file_path: &str,
    results: &mut Vec<Value>,
) {
    if node.kind() == kind {
        let start = node.start_position();
        let end = node.end_position();
        let text = get_node_text(node, lines);
        results.push(json!({
            "kind": kind,
            "text": text,
            "file": file_path,
            "start_line": start.row,
            "start_col": start.column,
            "end_line": end.row,
            "end_col": end.column,
        }));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_nodes_by_kind(child, kind, lines, file_path, results);
    }
}

fn find_text_in_nodes(
    node: Node,
    pattern: &str,
    lines: &[&str],
    file_path: &str,
    results: &mut Vec<Value>,
) {
    let text = get_node_text(node, lines);
    if text.contains(pattern) {
        let start = node.start_position();
        let end = node.end_position();
        results.push(json!({
            "kind": node.kind(),
            "text": text,
            "file": file_path,
            "start_line": start.row,
            "start_col": start.column,
            "end_line": end.row,
            "end_col": end.column,
        }));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_text_in_nodes(child, pattern, lines, file_path, results);
    }
}

fn get_node_text(node: Node, lines: &[&str]) -> String {
    let start = node.start_position();
    let end = node.end_position();

    if start.row == end.row {
        lines
            .get(start.row)
            .map(|line| {
                let s = start.column.min(line.len());
                let e = end.column.min(line.len());
                line[s..e].to_string()
            })
            .unwrap_or_default()
    } else {
        let mut result = String::new();
        for i in start.row..=end.row.min(lines.len().saturating_sub(1)) {
            if let Some(line) = lines.get(i) {
                if i == start.row {
                    result.push_str(&line[start.column.min(line.len())..]);
                } else if i == end.row {
                    result.push_str(&line[..end.column.min(line.len())]);
                } else {
                    result.push_str(line);
                }
                if i < end.row {
                    result.push('\n');
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_support::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn finds_rust_functions() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn main() {}\nfn helper() {}\n")
            .await
            .unwrap();

        let result = AstGrepTool
            .execute(json!({ "path": file.to_string_lossy(), "pattern": "fn $NAME" }))
            .await
            .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert!(matches.len() >= 2, "Expected at least 2 matches, got {}", matches.len());
    }

    #[tokio::test]
    async fn finds_structs() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "struct Foo { x: i32 }\nstruct Bar;\n")
            .await
            .unwrap();

        let result = AstGrepTool
            .execute(json!({ "path": file.to_string_lossy(), "pattern": "struct $NAME" }))
            .await
            .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert!(matches.len() >= 2);
    }
}
