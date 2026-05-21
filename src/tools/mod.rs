use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

pub mod fs;
pub mod hashline;
pub mod code_intel;
pub mod self_evolve;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// JSON Schema for tool parameters (optional, defaults to empty object)
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, input: Value) -> Result<Value>;
}

pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register(Box::new(fs::ReadFileTool));
        registry.register(Box::new(fs::WriteFileTool));
        registry.register(Box::new(fs::EditFileTool));
        registry.register(Box::new(code_intel::AstOutlineTool));
        registry.register(Box::new(code_intel::AstFunctionsTool));
        registry.register(Box::new(code_intel::AstImportsTool));
        registry.register(Box::new(code_intel::LspDefinitionTool));
        registry.register(Box::new(code_intel::LspReferencesTool));
        registry.register(Box::new(code_intel::LspHoverTool));
        registry.register(Box::new(self_evolve::ReadSelfTool));
        registry.register(Box::new(self_evolve::EditSelfTool));
        registry.register(Box::new(self_evolve::CargoTool));
        registry.register(Box::new(self_evolve::GitTool));
        registry.register(Box::new(self_evolve::SearchCodeTool));
        registry.register(Box::new(self_evolve::WorkspaceRootTool));
        registry.register(Box::new(self_evolve::RecordModificationTool));
        registry.register(Box::new(self_evolve::ModificationStatsTool));
        registry.register(Box::new(self_evolve::ReflectModificationTool));
        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    pub fn describe_all(&self) -> String {
        let mut descriptions: Vec<&str> = self.tools.values().map(|t| t.description()).collect();
        descriptions.sort();
        descriptions.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_code_intelligence_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.get("ast_outline").is_some());
        assert!(registry.get("ast_functions").is_some());
        assert!(registry.get("ast_imports").is_some());
        assert!(registry.get("lsp_definition").is_some());
        assert!(registry.get("lsp_references").is_some());
        assert!(registry.get("lsp_hover").is_some());
    }

    #[test]
    fn registers_self_evolution_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.get("read_self").is_some());
        assert!(registry.get("edit_self").is_some());
        assert!(registry.get("cargo").is_some());
        assert!(registry.get("git").is_some());
        assert!(registry.get("search_code").is_some());
        assert!(registry.get("workspace_root").is_some());
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
pub use self_evolve::register_self_evolve_tools;
