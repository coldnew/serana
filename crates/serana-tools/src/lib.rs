use std::collections::HashMap;

use serana_core::Tool;

pub mod code_intel;
pub mod eval;
pub mod fs;
pub mod git_ops;
pub mod github;
pub mod hashline;
pub mod memory;
pub mod recipe;
pub mod self_evolve;
pub mod ssh;
pub mod web_search;

pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        // File tools
        registry.register(Box::new(fs::ReadFileTool));
        registry.register(Box::new(fs::WriteFileTool));
        registry.register(Box::new(fs::EditFileTool));
        // Code intelligence tools
        registry.register(Box::new(code_intel::AstOutlineTool));
        registry.register(Box::new(code_intel::AstFunctionsTool));
        registry.register(Box::new(code_intel::AstImportsTool));
        registry.register(Box::new(code_intel::LspDefinitionTool));
        registry.register(Box::new(code_intel::LspReferencesTool));
        registry.register(Box::new(code_intel::LspHoverTool));
        // Self-evolution tools
        registry.register(Box::new(self_evolve::ReadSelfTool));
        registry.register(Box::new(self_evolve::EditSelfTool));
        registry.register(Box::new(self_evolve::CargoTool));
        registry.register(Box::new(self_evolve::GitTool));
        registry.register(Box::new(self_evolve::SearchCodeTool));
        registry.register(Box::new(self_evolve::WorkspaceRootTool));
        registry.register(Box::new(self_evolve::RecordModificationTool));
        registry.register(Box::new(self_evolve::ModificationStatsTool));
        registry.register(Box::new(self_evolve::ReflectModificationTool));
        // Eval tool
        registry.register(Box::new(eval::EvalTool));
        // Web search tools
        registry.register(Box::new(web_search::WebSearchTool));
        registry.register(Box::new(web_search::UrlFetchTool));
        // GitHub tools
        registry.register(Box::new(github::GitHubPrViewTool));
        registry.register(Box::new(github::GitHubIssueViewTool));
        registry.register(Box::new(github::GitHubPrDiffTool));
        // SSH tool
        registry.register(Box::new(ssh::SshTool));
        // Recipe tool
        registry.register(Box::new(recipe::RecipeTool));
        // Git operations tools
        registry.register(Box::new(git_ops::GitStatusTool));
        registry.register(Box::new(git_ops::GitDiffTool));
        registry.register(Box::new(git_ops::GitLogTool));
        registry.register(Box::new(git_ops::GitCommitTool));
        // Memory tools (init store, register tools)
        if let Err(e) = memory::register_memory_tools(&mut registry) {
            tracing::warn!("Failed to initialize memory store: {}", e);
        }
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

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub use self_evolve::register_self_evolve_tools;

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
        assert!(registry.get("verify_self").is_some());
        assert!(registry.get("git").is_some());
        assert!(registry.get("search_code").is_some());
        assert!(registry.get("workspace_root").is_some());
    }

    #[test]
    fn registers_new_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.get("eval").is_some());
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("url_fetch").is_some());
        assert!(registry.get("github_pr_view").is_some());
        assert!(registry.get("github_issue_view").is_some());
        assert!(registry.get("github_pr_diff").is_some());
        assert!(registry.get("ssh").is_some());
        assert!(registry.get("recipe").is_some());
        assert!(registry.get("git_status").is_some());
        assert!(registry.get("git_diff").is_some());
        assert!(registry.get("git_log").is_some());
        assert!(registry.get("git_commit").is_some());
        assert!(registry.get("retain").is_some());
        assert!(registry.get("recall").is_some());
        assert!(registry.get("reflect").is_some());
    }
}
