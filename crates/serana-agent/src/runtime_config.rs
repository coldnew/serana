use std::path::PathBuf;

use serana_tools::ToolProfile;

use crate::ContextCompressor;

#[derive(Clone)]
pub struct AgentRuntimeConfig {
    pub workspace: PathBuf,
    pub skills: Vec<String>,
    pub compressor: ContextCompressor,
    pub tool_profile: ToolProfile,
}

#[derive(Clone)]
pub struct AgentPromptConfig {
    pub workspace: PathBuf,
    pub skills: Vec<String>,
    pub compressor: ContextCompressor,
}

impl AgentRuntimeConfig {
    #[deprecated(note = "Use AgentRuntimeConfig::hermes to name the default Hermes policy.")]
    pub fn new(workspace: PathBuf) -> Self {
        Self::hermes(workspace)
    }

    pub fn hermes(workspace: PathBuf) -> Self {
        Self {
            workspace,
            skills: Vec::new(),
            compressor: ContextCompressor::with_defaults(),
            tool_profile: ToolProfile::Hermes,
        }
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    pub fn with_compressor(mut self, compressor: ContextCompressor) -> Self {
        self.compressor = compressor;
        self
    }

    pub fn with_tool_profile(mut self, profile: ToolProfile) -> Self {
        self.tool_profile = profile;
        self
    }

    pub fn prompt_config(&self) -> AgentPromptConfig {
        AgentPromptConfig {
            workspace: self.workspace.clone(),
            skills: self.skills.clone(),
            compressor: self.compressor.clone(),
        }
    }
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self::hermes(PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_hermes_profile() {
        let config = AgentRuntimeConfig::default();

        assert_eq!(config.tool_profile, ToolProfile::Hermes);
    }

    #[test]
    fn hermes_constructor_names_default_policy() {
        let config = AgentRuntimeConfig::hermes(PathBuf::from("workspace"));

        assert_eq!(config.workspace, PathBuf::from("workspace"));
        assert_eq!(config.tool_profile, ToolProfile::Hermes);
    }
}
