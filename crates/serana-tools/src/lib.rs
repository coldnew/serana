use std::collections::HashMap;

use serana_core::{FunctionDefinition, Tool, ToolDefinition};

pub mod ast_edit;
pub mod browser;
pub mod checkpoint;
pub mod clipboard;
pub mod code_intel;
pub mod conflict;
pub mod dap;
pub mod eval;
pub mod fs;
pub mod git_ops;
pub mod github;
pub mod hashline;
pub mod mcp;
pub mod memory;
pub mod recipe;
pub mod review;
pub mod rules;
pub mod search_native;
pub mod self_evolve;
pub mod shell;
pub mod skill;
pub mod ssh;
pub mod stats;
pub mod web_search;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolProfile {
    Core,
    Workspace,
    #[deprecated(note = "Use ToolProfile::Workspace for the non-Hermes workspace tool profile.")]
    Coding,
    Hermes,
}

pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    #[deprecated(
        note = "Use ToolRegistry::hermes, ToolRegistry::workspace, or ToolRegistry::core to name the intended tool power level."
    )]
    pub fn new() -> Self {
        Self::hermes()
    }

    pub fn core() -> Self {
        Self::for_profile(ToolProfile::Core)
    }

    pub fn workspace() -> Self {
        Self::for_profile(ToolProfile::Workspace)
    }

    #[deprecated(note = "Use ToolRegistry::workspace for the non-Hermes workspace tool profile.")]
    pub fn coding() -> Self {
        Self::workspace()
    }

    pub fn hermes() -> Self {
        Self::for_profile(ToolProfile::Hermes)
    }

    pub fn for_profile(profile: ToolProfile) -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register_core_tools();
        if profile.includes_workspace_tools() {
            registry.register_workspace_tools();
        }
        if profile.includes_hermes_tools() {
            registry.register_hermes_tools();
        }
        registry
    }

    fn register_core_tools(&mut self) {
        self.register(Box::new(fs::ReadFileTool));
        self.register(Box::new(fs::WriteFileTool));
        self.register(Box::new(fs::EditFileTool));
        self.register(Box::new(code_intel::AstOutlineTool));
        self.register(Box::new(code_intel::AstFunctionsTool));
        self.register(Box::new(code_intel::AstImportsTool));
        self.register(Box::new(ast_edit::AstEditTool));
        self.register(Box::new(search_native::FindTool));
        self.register(Box::new(search_native::SearchTool));
        self.register(Box::new(rules::RulesInfoTool));
    }

    fn register_workspace_tools(&mut self) {
        self.register(Box::new(git_ops::GitStatusTool));
        self.register(Box::new(git_ops::GitDiffTool));
        self.register(Box::new(git_ops::GitLogTool));
        self.register(Box::new(git_ops::GitCommitTool));
        self.register(Box::new(checkpoint::CheckpointTool));
        self.register(Box::new(checkpoint::RewindTool));
        self.register(Box::new(review::CodeReviewTool));
        self.register(Box::new(conflict::ConflictResolveTool));
        self.register(Box::new(stats::StatsTool::new()));
        self.register(Box::new(shell::ShellTool::new()));
        if let Err(e) = memory::register_memory_tools(self) {
            tracing::warn!("Failed to initialize memory store: {}", e);
        }
    }

    fn register_hermes_tools(&mut self) {
        self.register(Box::new(self_evolve::ReadSelfTool));
        self.register(Box::new(self_evolve::EditSelfTool));
        self.register(Box::new(self_evolve::CargoTool));
        self.register(Box::new(self_evolve::VerifySelfTool));
        self.register(Box::new(self_evolve::GitTool));
        self.register(Box::new(self_evolve::SearchCodeTool));
        self.register(Box::new(self_evolve::WorkspaceRootTool));
        self.register(Box::new(self_evolve::RecordModificationTool));
        self.register(Box::new(self_evolve::ModificationStatsTool));
        self.register(Box::new(self_evolve::ReflectModificationTool));
        self.register(Box::new(eval::EvalTool));
        self.register(Box::new(web_search::WebSearchTool));
        self.register(Box::new(web_search::UrlFetchTool));
        self.register(Box::new(github::GitHubPrViewTool));
        self.register(Box::new(github::GitHubIssueViewTool));
        self.register(Box::new(github::GitHubPrDiffTool));
        self.register(Box::new(ssh::SshTool));
        self.register(Box::new(recipe::RecipeTool));
        self.register(Box::new(dap::DebugTool::new()));
        self.register(Box::new(browser::BrowserTool));
        self.register(Box::new(mcp::McpTool::new()));
        self.register(Box::new(clipboard::ClipboardCopyTool));
        self.register(Box::new(clipboard::ClipboardPasteTool));
    }

    /// Register LSP tools with a shared, persistent LspManager.
    pub fn register_lsp(&mut self, manager: code_intel::SharedLspManager) {
        self.register(Box::new(code_intel::LspDefinitionTool {
            manager: manager.clone(),
        }));
        self.register(Box::new(code_intel::LspReferencesTool {
            manager: manager.clone(),
        }));
        self.register(Box::new(code_intel::LspHoverTool { manager }));
    }
    /// Register skill creation tool.
    pub fn register_skill(&mut self, workspace: std::path::PathBuf) {
        self.register(Box::new(skill::SkillCreateTool::new(workspace)));
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

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                r#type: "function".to_string(),
                function: FunctionDefinition {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters(),
                },
            })
            .collect()
    }

    pub fn describe_all(&self) -> String {
        let mut descriptions: Vec<&str> = self.tools.values().map(|t| t.description()).collect();
        descriptions.sort();
        descriptions.join("\n")
    }
}

impl ToolProfile {
    #[allow(deprecated)]
    fn includes_workspace_tools(self) -> bool {
        matches!(
            self,
            ToolProfile::Workspace | ToolProfile::Coding | ToolProfile::Hermes
        )
    }

    fn includes_hermes_tools(self) -> bool {
        matches!(self, ToolProfile::Hermes)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::hermes()
    }
}

pub use self_evolve::register_self_evolve_tools;

#[cfg(test)]
mod test_support {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    pub fn tempdir() -> std::io::Result<TempDir> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path =
            std::env::temp_dir().join(format!("serana-tools-{}-{}", std::process::id(), suffix));
        std::fs::create_dir_all(&path)?;
        Ok(TempDir { path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_code_intelligence_tools() {
        let registry = ToolRegistry::core();
        assert!(registry.get("ast_outline").is_some());
        assert!(registry.get("ast_functions").is_some());
        assert!(registry.get("ast_imports").is_some());
        assert!(registry.get("ast_edit").is_some());
    }

    #[test]
    fn registers_self_evolution_tools() {
        let registry = ToolRegistry::hermes();
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
        let registry = ToolRegistry::hermes();
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
        assert!(registry.get("debug").is_some());
        assert!(registry.get("browser").is_some());
        assert!(registry.get("conflict_resolve").is_some());
        assert!(registry.get("checkpoint").is_some());
        assert!(registry.get("rewind").is_some());
        assert!(registry.get("code_review").is_some());
        assert!(registry.get("rules_info").is_some());
        assert!(registry.get("stats").is_some());
        assert!(registry.get("mcp").is_some());
        assert!(registry.get("clipboard_copy").is_some());
        assert!(registry.get("clipboard_paste").is_some());
        assert!(registry.get("shell").is_some());
        assert!(registry.get("find").is_some());
        assert!(registry.get("search").is_some());
    }

    #[test]
    fn core_profile_excludes_high_power_tools() {
        let registry = ToolRegistry::core();
        assert!(registry.get("read_file").is_some());
        assert!(registry.get("ast_outline").is_some());
        assert!(registry.get("find").is_some());
        assert!(registry.get("read_self").is_none());
        assert!(registry.get("ssh").is_none());
        assert!(registry.get("web_search").is_none());
    }

    #[test]
    fn workspace_profile_has_workspace_tools_without_hermes_tools() {
        let registry = ToolRegistry::workspace();
        assert!(registry.get("git_status").is_some());
        assert!(registry.get("shell").is_some());
        assert!(registry.get("checkpoint").is_some());
        assert!(registry.get("read_self").is_none());
        assert!(registry.get("ssh").is_none());
        assert!(registry.get("mcp").is_none());
    }

    #[test]
    #[allow(deprecated)]
    fn coding_profile_remains_workspace_compatibility() {
        let registry = ToolRegistry::for_profile(ToolProfile::Coding);
        assert!(registry.get("git_status").is_some());
        assert!(registry.get("read_self").is_none());

        let registry = ToolRegistry::coding();
        assert!(registry.get("git_status").is_some());
        assert!(registry.get("read_self").is_none());
    }

    #[test]
    fn hermes_profile_enables_self_evolution_and_external_tools() {
        let registry = ToolRegistry::hermes();
        assert!(registry.get("read_self").is_some());
        assert!(registry.get("verify_self").is_some());
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("ssh").is_some());
        assert!(registry.get("mcp").is_some());
    }

    #[test]
    fn exposes_tool_definitions_for_model_calls() {
        let registry = ToolRegistry::core();
        let definitions = registry.definitions();

        assert!(definitions.iter().any(|definition| {
            definition.function.name == "read_file"
                && definition.r#type == "function"
                && !definition.function.description.is_empty()
        }));
    }

    #[test]
    #[allow(deprecated)]
    fn new_registry_is_powered_hermes() {
        let registry = ToolRegistry::new();

        assert!(registry.get("read_self").is_some());
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("ssh").is_some());
    }
}
