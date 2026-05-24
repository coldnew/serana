use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PromptBuilder {
    workspace: PathBuf,
    personality_path: Option<PathBuf>,
    memory_path: Option<PathBuf>,
    user_memory_path: Option<PathBuf>,
    skills: Vec<String>,
    tool_descriptions: String,
}

impl PromptBuilder {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            personality_path: None,
            memory_path: None,
            user_memory_path: None,
            skills: Vec::new(),
            tool_descriptions: String::new(),
        }
    }

    pub fn with_personality(mut self, path: PathBuf) -> Self {
        self.personality_path = Some(path);
        self
    }

    pub fn with_memory(mut self, path: PathBuf) -> Self {
        self.memory_path = Some(path);
        self
    }

    pub fn with_user_memory(mut self, path: PathBuf) -> Self {
        self.user_memory_path = Some(path);
        self
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    pub fn with_tool_descriptions(mut self, descriptions: String) -> Self {
        self.tool_descriptions = descriptions;
        self
    }

    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        parts.push(self.build_core_prompt());

        if let Some(path) = &self.personality_path {
            if let Some(content) = self.read_file(path) {
                parts.push(format!("\n## Personality\n\n{}", content));
            }
        }

        if let Some(path) = &self.memory_path {
            if let Some(content) = self.read_file(path) {
                parts.push(format!("\n## Memory (Project)\n\n{}", content));
            }
        }

        if let Some(path) = &self.user_memory_path {
            if let Some(content) = self.read_file(path) {
                parts.push(format!("\n## Memory (User)\n\n{}", content));
            }
        }

        if !self.skills.is_empty() {
            parts.push(self.build_skills_section());
        }

        parts.push(self.build_context_files_section());

        if !self.tool_descriptions.is_empty() {
            parts.push(format!(
                "\n## Available Tools\n\n{}",
                self.tool_descriptions
            ));
        }

        parts.push(self.build_tool_guidance());

        parts.join("\n")
    }

    fn build_core_prompt(&self) -> String {
        let ws_name = self
            .workspace
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");

        format!(
            r#"You are Serana, a Hermes agent that helps with programming tasks.

## Core Capabilities
- Read, write, and edit files in the workspace
- Execute shell commands and capture output
- Search code using LSP and tree-sitter
- Answer questions about code structure

## Workspace
Current workspace: {}

## Guidelines
1. Think step by step before taking actions
2. Use tools to gather information before answering
3. Explain what you're doing and why
4. When editing files, make minimal, precise changes
5. Verify your changes work before finishing

## Communication
- Be concise but informative
- Show your reasoning process
- Ask for clarification when needed
- Acknowledge when you don't know something"#,
            ws_name
        )
    }

    fn build_skills_section(&self) -> String {
        let mut section = String::from("\n## Skills\n\n");
        for skill in &self.skills {
            section.push_str(&format!("- {}\n", skill));
        }
        section
    }

    fn build_context_files_section(&self) -> String {
        let mut parts = Vec::new();

        let agents_path = self.workspace.join("AGENTS.md");
        if let Some(content) = self.read_file(&agents_path) {
            parts.push(format!(
                "\n## Project Guidelines (AGENTS.md)\n\n{}",
                content
            ));
        }

        let serana_path = self.workspace.join(".serana.md");
        if let Some(content) = self.read_file(&serana_path) {
            parts.push(format!("\n## Project Context (.serana.md)\n\n{}", content));
        }

        let hermes_path = self.workspace.join(".hermes.md");
        if let Some(content) = self.read_file(&hermes_path) {
            parts.push(format!("\n## Project Context (.hermes.md)\n\n{}", content));
        }

        parts.join("\n")
    }

    fn build_tool_guidance(&self) -> String {
        r#"
## Tool Usage Guidance

When using tools, follow these patterns:
- **read_file**: Use to examine code before making changes
- **write_file**: Create new files or overwrite existing ones
- **edit_file**: Use the hashline patch format for precise edits
- **bash**: Execute commands; capture output for analysis

For edits, prefer the hashline format which uses line-anchored patches:
```
@@ src/file.rs
= 41th..57rk
~/// New comment
~pub fn new_function() {}
```

Always verify file contents after editing to ensure changes were applied correctly."#
            .to_string()
    }

    fn read_file(&self, path: &Path) -> Option<String> {
        fs::read_to_string(path).ok()
    }
}

pub fn build_task_prompt(task: &str, context: &str) -> String {
    format!(
        r#"## Task
{}

## Context
{}

Please complete the task using the available tools. Show your reasoning and explain what you're doing."#,
        task, context
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tempdir;

    #[test]
    fn test_prompt_builder() {
        let dir = tempdir().unwrap();
        let builder = PromptBuilder::new(dir.path().to_path_buf());
        let prompt = builder.build();

        assert!(prompt.contains("Serana"));
        assert!(prompt.contains("Hermes agent"));
    }

    #[test]
    fn test_prompt_with_memory() {
        let dir = tempdir().unwrap();
        let memory_path = dir.path().join("MEMORY.md");
        fs::write(&memory_path, "Project memory: use async patterns").unwrap();

        let builder = PromptBuilder::new(dir.path().to_path_buf()).with_memory(memory_path);
        let prompt = builder.build();

        assert!(prompt.contains("Project memory"));
        assert!(prompt.contains("async patterns"));
    }

    #[test]
    fn test_prompt_with_agents_md() {
        let dir = tempdir().unwrap();
        let agents_path = dir.path().join("AGENTS.md");
        fs::write(&agents_path, "Always run tests before committing").unwrap();

        let builder = PromptBuilder::new(dir.path().to_path_buf());
        let prompt = builder.build();

        assert!(prompt.contains("Always run tests"));
    }
}
