use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

use serana_core::{Result, Tool};
use crate::lsp::{types::Position, LspManager};
use serana_tree_sitter::ParserManager;

pub struct AstOutlineTool;
pub struct AstFunctionsTool;
pub struct AstImportsTool;

/// Shared LSP manager wrapped in Arc<Mutex<>> for persistence across tool calls.
pub type SharedLspManager = Arc<Mutex<LspManager>>;

/// Create a shared LSP manager for a given workspace.
pub fn new_shared_lsp_manager(workspace: std::path::PathBuf) -> SharedLspManager {
    Arc::new(Mutex::new(LspManager::new(workspace)))
}

pub struct LspDefinitionTool {
    pub manager: SharedLspManager,
}
pub struct LspReferencesTool {
    pub manager: SharedLspManager,
}
pub struct LspHoverTool {
    pub manager: SharedLspManager,
}
pub struct LspDiagnosticsTool {
    pub manager: SharedLspManager,
}
pub struct LspCodeActionTool {
    pub manager: SharedLspManager,
}
pub struct LspRenameTool {
    pub manager: SharedLspManager,
}
pub struct LspFormatTool {
    pub manager: SharedLspManager,
}
pub struct LspDocumentSymbolsTool {
    pub manager: SharedLspManager,
}
pub struct LspWorkspaceSymbolsTool {
    pub manager: SharedLspManager,
}
pub struct LspCompletionTool {
    pub manager: SharedLspManager,
}
pub struct LspSignatureHelpTool {
    pub manager: SharedLspManager,
}
pub struct LspImplementationTool {
    pub manager: SharedLspManager,
}

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
        let mut mgr = self.manager.lock().await;
        let locations = mgr
            .definition(request.path.as_ref(), request.position)
            .await?;
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
        let mut mgr = self.manager.lock().await;
        let locations = mgr
            .references(request.path.as_ref(), request.position)
            .await?;
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
        let mut mgr = self.manager.lock().await;
        let hover = mgr.hover(request.path.as_ref(), request.position).await?;
        Ok(json!({ "hover": hover }))
    }
}

#[async_trait]
impl Tool for LspDiagnosticsTool {
    fn name(&self) -> &'static str {
        "lsp_diagnostics"
    }
    fn description(&self) -> &'static str {
        "Use a language server to get diagnostics (errors, warnings) for a file. Input: {\"path\": \"src/main.rs\"}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"}
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let mut mgr = self.manager.lock().await;
        let diagnostics = mgr.diagnostics(std::path::Path::new(path)).await?;
        Ok(json!({ "diagnostics": diagnostics }))
    }
}

#[async_trait]
impl Tool for LspCodeActionTool {
    fn name(&self) -> &'static str {
        "lsp_code_action"
    }
    fn description(&self) -> &'static str {
        "Use a language server to get code actions (quick fixes, refactors) for a range. Input: {\"path\": \"src/main.rs\", \"start_line\": 0, \"start_char\": 0, \"end_line\": 0, \"end_char\": 10}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "start_line": {"type": "integer", "description": "0-based start line"},
                "start_char": {"type": "integer", "description": "0-based start character"},
                "end_line": {"type": "integer", "description": "0-based end line"},
                "end_char": {"type": "integer", "description": "0-based end character"}
            },
            "required": ["path", "start_line", "start_char", "end_line", "end_char"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let start_line = input.get("start_line").and_then(Value::as_u64).unwrap_or(0) as u32;
        let start_char = input.get("start_char").and_then(Value::as_u64).unwrap_or(0) as u32;
        let end_line = input.get("end_line").and_then(Value::as_u64).unwrap_or(0) as u32;
        let end_char = input.get("end_char").and_then(Value::as_u64).unwrap_or(0) as u32;
        let mut mgr = self.manager.lock().await;
        let actions = mgr.code_action(
            std::path::Path::new(path),
            Position { line: start_line, character: start_char },
            Position { line: end_line, character: end_char },
        ).await?;
        Ok(json!({ "actions": actions }))
    }
}

#[async_trait]
impl Tool for LspRenameTool {
    fn name(&self) -> &'static str {
        "lsp_rename"
    }
    fn description(&self) -> &'static str {
        "Use a language server to rename a symbol across the workspace. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5, \"new_name\": \"new_name\"}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"},
                "new_name": {"type": "string", "description": "New name for the symbol"}
            },
            "required": ["path", "line", "character", "new_name"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let new_name = input.get("new_name").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_name' field"))?;
        let mut mgr = self.manager.lock().await;
        let edit = mgr.rename(request.path.as_ref(), request.position, new_name).await?;
        Ok(json!({ "edit": edit }))
    }
}

#[async_trait]
impl Tool for LspFormatTool {
    fn name(&self) -> &'static str {
        "lsp_format"
    }
    fn description(&self) -> &'static str {
        "Use a language server to format a source file. Input: {\"path\": \"src/main.rs\"}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"}
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let mut mgr = self.manager.lock().await;
        let edits = mgr.formatting(std::path::Path::new(path)).await?;
        Ok(json!({ "edits": edits }))
    }
}

#[async_trait]
impl Tool for LspDocumentSymbolsTool {
    fn name(&self) -> &'static str {
        "lsp_document_symbols"
    }
    fn description(&self) -> &'static str {
        "Use a language server to list all symbols in a file. Input: {\"path\": \"src/main.rs\"}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"}
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;
        let mut mgr = self.manager.lock().await;
        let symbols = mgr.document_symbols(std::path::Path::new(path)).await?;
        Ok(json!({ "symbols": symbols }))
    }
}

#[async_trait]
impl Tool for LspWorkspaceSymbolsTool {
    fn name(&self) -> &'static str {
        "lsp_workspace_symbols"
    }
    fn description(&self) -> &'static str {
        "Use a language server to search symbols across the workspace. Input: {\"query\": \"MyStruct\"}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query for symbol names"}
            },
            "required": ["query"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let query = input.get("query").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' field"))?;
        let mut mgr = self.manager.lock().await;
        let symbols = mgr.workspace_symbols(query).await?;
        Ok(json!({ "symbols": symbols }))
    }
}

#[async_trait]
impl Tool for LspCompletionTool {
    fn name(&self) -> &'static str {
        "lsp_completion"
    }
    fn description(&self) -> &'static str {
        "Use a language server to get code completions at a position. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut mgr = self.manager.lock().await;
        let completions = mgr.completion(request.path.as_ref(), request.position).await?;
        Ok(json!({ "completions": completions }))
    }
}

#[async_trait]
impl Tool for LspSignatureHelpTool {
    fn name(&self) -> &'static str {
        "lsp_signature_help"
    }
    fn description(&self) -> &'static str {
        "Use a language server to get function signature help at a position. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 10}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut mgr = self.manager.lock().await;
        let sig = mgr.signature_help(request.path.as_ref(), request.position).await?;
        Ok(json!({ "signature": sig }))
    }
}

#[async_trait]
impl Tool for LspImplementationTool {
    fn name(&self) -> &'static str {
        "lsp_implementation"
    }
    fn description(&self) -> &'static str {
        "Use a language server to find implementations of an interface/trait. Input: {\"path\": \"src/main.rs\", \"line\": 0, \"character\": 5}"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the source file"},
                "line": {"type": "integer", "description": "0-based line number"},
                "character": {"type": "integer", "description": "0-based character offset"}
            },
            "required": ["path", "line", "character"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let request = LspToolRequest::from_input(&input)?;
        let mut mgr = self.manager.lock().await;
        let locations = mgr.implementation(request.path.as_ref(), request.position).await?;
        Ok(json!({ "locations": locations }))
    }
}

struct LspToolRequest {
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
        Ok(Self {
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
    use crate::tools::test_support::tempdir;

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
