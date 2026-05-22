use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use serana_core::{Result, Tool};
use serana_lsp::{types::Position, LspManager};
use serana_tree_sitter::ParserManager;

pub struct AstOutlineTool;
pub struct AstFunctionsTool;
pub struct AstImportsTool;
pub struct LspDefinitionTool;
pub struct LspReferencesTool;
pub struct LspHoverTool;

#[async_trait]
impl Tool for AstOutlineTool {
    fn name(&self) -> &'static str {
        "ast_outline"
    }

    fn description(&self) -> &'static str {
        "Parse a source file with tree-sitter and return functions, types/classes, and imports. Input: {\"path\": \"src/main.rs\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the source file to parse"
                }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let (path, content) = read_source(input).await?;
        let manager = ParserManager::new();
        let tree = manager.parse_file(path.as_ref(), &content)?;
        Ok(json!({
            "path": path,
            "functions": manager.query_functions(&tree, &content)?,
            "types": manager.query_structs(&tree, &content)?,
            "imports": manager.query_imports(&tree, &content)?,
        }))
    }
}

#[async_trait]
impl Tool for AstFunctionsTool {
    fn name(&self) -> &'static str {
        "ast_functions"
    }

    fn description(&self) -> &'static str {
        "Parse a source file with tree-sitter and return function/method definitions. Input: {\"path\": \"src/main.rs\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the source file to parse"
                }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let (path, content) = read_source(input).await?;
        let manager = ParserManager::new();
        let tree = manager.parse_file(path.as_ref(), &content)?;
        Ok(json!({
            "path": path,
            "functions": manager.query_functions(&tree, &content)?,
        }))
    }
}

#[async_trait]
impl Tool for AstImportsTool {
    fn name(&self) -> &'static str {
        "ast_imports"
    }

    fn description(&self) -> &'static str {
        "Parse a source file with tree-sitter and return import/use statements. Input: {\"path\": \"src/main.rs\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the source file to parse"
                }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let (path, content) = read_source(input).await?;
        let manager = ParserManager::new();
        let tree = manager.parse_file(path.as_ref(), &content)?;
        Ok(json!({
            "path": path,
            "imports": manager.query_imports(&tree, &content)?,
        }))
    }
}

#[async_trait]
impl Tool for LspDefinitionTool {
    fn name(&self) -> &'static str {
        "lsp_definition"
    }

    fn description(&self) -> &'static str {
        "Use a language server to find definition locations. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5} (line/character are 0-based)"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"},
                "workspace": {"type": "string", "description": "Workspace root (optional)"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut manager = LspManager::new(request.workspace);
        let locations = manager
            .definition(request.path.as_ref(), request.position)
            .await?;
        manager.shutdown_all().await?;
        Ok(json!({ "locations": locations }))
    }
}

#[async_trait]
impl Tool for LspReferencesTool {
    fn name(&self) -> &'static str {
        "lsp_references"
    }

    fn description(&self) -> &'static str {
        "Use a language server to find references. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5} (line/character are 0-based)"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"},
                "workspace": {"type": "string", "description": "Workspace root (optional)"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut manager = LspManager::new(request.workspace);
        let locations = manager
            .references(request.path.as_ref(), request.position)
            .await?;
        manager.shutdown_all().await?;
        Ok(json!({ "locations": locations }))
    }
}

#[async_trait]
impl Tool for LspHoverTool {
    fn name(&self) -> &'static str {
        "lsp_hover"
    }

    fn description(&self) -> &'static str {
        "Use a language server to get hover/type info. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5} (line/character are 0-based)"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"},
                "workspace": {"type": "string", "description": "Workspace root (optional)"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut manager = LspManager::new(request.workspace);
        let hover = manager
            .hover(request.path.as_ref(), request.position)
            .await?;
        manager.shutdown_all().await?;
        Ok(json!({ "hover": hover }))
    }
}

struct LspToolRequest {
    workspace: std::path::PathBuf,
    path: String,
    position: Position,
}

impl LspToolRequest {
    fn from_input(input: &Value) -> Result<Self> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?
            .to_string();
        let line = input
            .get("line")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("Missing 'line' field"))? as u32;
        let character = input
            .get("character")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("Missing 'character' field"))?
            as u32;
        let workspace = input
            .get("workspace")
            .and_then(Value::as_str)
            .map(std::path::PathBuf::from)
            .unwrap_or(std::env::current_dir()?);
        Ok(Self {
            workspace,
            path,
            position: Position { line, character },
        })
    }
}

async fn read_source(input: Value) -> Result<(String, String)> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
    let content = fs::read_to_string(path).await?;
    Ok((path.to_string(), content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn ast_outline_returns_rust_symbols() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "use std::fmt;\nstruct App;\nfn main() {}\n")
            .await
            .unwrap();

        let output = AstOutlineTool
            .execute(json!({ "path": file.to_string_lossy() }))
            .await
            .unwrap();

        assert_eq!(output["functions"][0]["name"], "main");
        assert_eq!(output["types"][0]["name"], "App");
        assert_eq!(output["imports"][0]["source"], "std::fmt");
    }

    #[test]
    fn parses_lsp_tool_request() {
        let request = LspToolRequest::from_input(&json!({
            "path": "src/main.rs",
            "line": 3,
            "character": 10,
            "workspace": "."
        }))
        .unwrap();
        assert_eq!(request.path, "src/main.rs");
        assert_eq!(request.position.line, 3);
        assert_eq!(request.position.character, 10);
    }
}
