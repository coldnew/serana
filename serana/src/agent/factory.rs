use crate::core::LlmClient;
use crate::tools::{ToolProfile, ToolRegistry};

use super::{AgentRuntimeConfig, HermesAgent};

#[derive(Clone)]
pub struct AgentFactory {
    runtime_config: AgentRuntimeConfig,
    enable_lsp: bool,
    enable_skill_tool: bool,
}

impl AgentFactory {
    #[deprecated(
        note = "Use AgentFactory::hermes for powered Hermes agents or AgentFactory::custom for raw tool policy assembly."
    )]
    pub fn new(runtime_config: AgentRuntimeConfig) -> Self {
        Self::custom(runtime_config)
    }

    pub fn custom(runtime_config: AgentRuntimeConfig) -> Self {
        Self {
            runtime_config,
            enable_lsp: false,
            enable_skill_tool: false,
        }
    }

    pub fn hermes(mut runtime_config: AgentRuntimeConfig) -> Self {
        runtime_config.tool_profile = ToolProfile::Hermes;
        Self::custom(runtime_config)
            .with_lsp_tools()
            .with_skill_tool()
    }

    pub fn with_lsp_tools(mut self) -> Self {
        self.enable_lsp = true;
        self
    }

    pub fn with_skill_tool(mut self) -> Self {
        self.enable_skill_tool = true;
        self
    }

    pub fn runtime_config(&self) -> &AgentRuntimeConfig {
        &self.runtime_config
    }

    pub fn build_tools(&self) -> ToolRegistry {
        let mut tools = match self.runtime_config.tool_profile {
            ToolProfile::Core => ToolRegistry::core(),
            ToolProfile::Workspace => ToolRegistry::workspace(),
            #[allow(deprecated)]
            ToolProfile::Coding => ToolRegistry::coding(),
            ToolProfile::Hermes => ToolRegistry::hermes(),
        };
        if self.enable_lsp {
            let manager = crate::tools::code_intel::new_shared_lsp_manager(
                self.runtime_config.workspace.clone(),
            );
            tools.register_lsp(manager);
        }
        if self.enable_skill_tool {
            tools.register_skill(self.runtime_config.workspace.clone());
        }
        tools
    }

    pub fn build(&self, llm: Box<dyn LlmClient>) -> HermesAgent {
        HermesAgent::with_tools(llm, self.build_tools())
            .with_prompt_config(self.runtime_config.prompt_config())
            .with_retry_config(self.runtime_config.retry_config.clone())
    }
}

impl Default for AgentFactory {
    fn default() -> Self {
        Self::hermes(AgentRuntimeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_tools_from_runtime_profile() {
        let factory = AgentFactory::custom(
            AgentRuntimeConfig::default().with_tool_profile(ToolProfile::Workspace),
        );
        let tools = factory.build_tools();

        assert!(tools.get("git_status").is_some());
        assert!(tools.get("read_self").is_none());
    }

    #[test]
    fn adds_workspace_extensions_explicitly() {
        let factory = AgentFactory::custom(AgentRuntimeConfig::default());
        let tools = factory.build_tools();

        assert!(tools.get("lsp_definition").is_none());
        assert!(tools.get("skill_create").is_none());

        let factory = factory.with_lsp_tools().with_skill_tool();
        let tools = factory.build_tools();

        assert!(tools.get("lsp_definition").is_some());
        assert!(tools.get("skill_create").is_some());
    }

    #[test]
    #[allow(deprecated)]
    fn new_remains_custom_factory_compatibility() {
        let factory = AgentFactory::new(AgentRuntimeConfig::default());
        let tools = factory.build_tools();

        assert_eq!(factory.runtime_config().tool_profile, ToolProfile::Hermes);
        assert!(tools.get("read_self").is_some());
        assert!(tools.get("lsp_definition").is_none());
        assert!(tools.get("skill_create").is_none());
    }

    #[test]
    fn hermes_factory_enables_default_power_tools() {
        let factory = AgentFactory::hermes(AgentRuntimeConfig::default());
        let tools = factory.build_tools();

        assert!(tools.get("read_self").is_some());
        assert!(tools.get("lsp_definition").is_some());
        assert!(tools.get("skill_create").is_some());
    }

    #[test]
    fn hermes_factory_forces_hermes_profile() {
        let factory = AgentFactory::hermes(
            AgentRuntimeConfig::default().with_tool_profile(ToolProfile::Core),
        );
        let tools = factory.build_tools();

        assert_eq!(factory.runtime_config().tool_profile, ToolProfile::Hermes);
        assert!(tools.get("read_self").is_some());
        assert!(tools.get("web_search").is_some());
    }

    #[test]
    fn default_factory_is_powered_hermes() {
        let factory = AgentFactory::default();
        let tools = factory.build_tools();

        assert_eq!(factory.runtime_config().tool_profile, ToolProfile::Hermes);
        assert!(tools.get("read_self").is_some());
        assert!(tools.get("lsp_definition").is_some());
        assert!(tools.get("skill_create").is_some());
    }
}
