//! Skill system for serana.
//!
//! Skills are reusable instructions loaded from markdown files.
//! Each skill lives in its own directory as `SKILL.md` with optional
//! YAML frontmatter for metadata.
//!
//! Search paths:
//!   1. `~/.serana/skills/<name>/SKILL.md`  (user-level)
//!   2. `.serana/skills/<name>/SKILL.md`    (project-level, relative to workspace)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::{Result, Tool};

/// A loaded skill.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub file_path: PathBuf,
    pub base_dir: PathBuf,
    pub source: SkillSource,
}

/// Where a skill was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    User,
    Project,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Project => write!(f, "project"),
        }
    }
}

/// Stores all loaded skills.
#[derive(Debug, Clone, Default)]
pub struct SkillStore {
    skills: HashMap<String, Skill>,
}

impl SkillStore {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Discover and load skills from standard locations.
    pub fn discover(workspace: &Path) -> Self {
        let mut store = Self::new();

        // User-level skills
        if let Some(home) = dirs::home_dir() {
            let user_skills = home.join(".serana").join("skills");
            store.scan_dir(&user_skills, SkillSource::User);
        }

        // Project-level skills
        let project_skills = workspace.join(".serana").join("skills");
        store.scan_dir(&project_skills, SkillSource::Project);

        store
    }

    /// Scan a directory for skill subdirectories containing SKILL.md.
    fn scan_dir(&mut self, dir: &Path, source: SkillSource) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            if let Some(skill) = load_skill_from_file(&skill_md, &source) {
                self.skills.insert(skill.name.clone(), skill);
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Build the skills section for the system prompt.
    pub fn render_for_prompt(&self) -> Option<String> {
        if self.skills.is_empty() {
            return None;
        }
        let mut skills: Vec<&Skill> = self.skills.values().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));

        let mut out = String::from("<skills>\n");
        for skill in &skills {
            out.push_str(&format!(
                "<skill name=\"{}\" source=\"{}\">\n{}</skill>\n",
                skill.name, skill.source, skill.body,
            ));
        }
        out.push_str("</skills>\n");
        Some(out)
    }

    /// Insert a skill into the store (used by skill_create tool).
    pub fn insert(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }
}

/// Load a single skill from a SKILL.md file.
fn load_skill_from_file(path: &Path, source: &SkillSource) -> Option<Skill> {
    let content = std::fs::read_to_string(path).ok()?;
    let (description, body) = parse_skill_md(&content);
    let base_dir = path.parent()?.to_path_buf();
    let name = base_dir.file_name()?.to_str()?.to_string();

    Some(Skill {
        name,
        description,
        body,
        file_path: path.to_path_buf(),
        base_dir,
        source: source.clone(),
    })
}

/// Parse SKILL.md content: extract YAML frontmatter description and body.
fn parse_skill_md(content: &str) -> (String, String) {
    let trimmed = content.trim();
    if trimmed.starts_with("---") {
        // Find closing ---
        if let Some(end) = trimmed[3..].find("\n---") {
            let frontmatter = &trimmed[3..3 + end].trim();
            let body_start = 3 + end + 4; // skip "\n---"
            let body = trimmed[body_start..].trim().to_string();
            let description = extract_frontmatter_field(frontmatter, "description");
            return (description, body);
        }
    }
    // No frontmatter — entire content is the body, description from first line
    let first_line = trimmed.lines().next().unwrap_or("").trim();
    let desc = first_line.trim_start_matches('#').trim().to_string();
    (desc, trimmed.to_string())
}

/// Extract a simple `key: value` field from YAML frontmatter (no quoting support needed).
fn extract_frontmatter_field(fm: &str, key: &str) -> String {
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start().strip_prefix(':').unwrap_or("").trim();
            // Strip surrounding quotes
            let val = rest
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('\'')
                .trim_end_matches('\'')
                .to_string();
            if !val.is_empty() {
                return val;
            }
        }
    }
    String::new()
}

// ── skill_create tool ──────────────────────────────────────────────

/// Tool that lets the agent create new skills.
pub struct SkillCreateTool {
    pub workspace: PathBuf,
}

impl SkillCreateTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for SkillCreateTool {
    fn name(&self) -> &'static str {
        "skill_create"
    }

    fn description(&self) -> &'static str {
        "Create a reusable skill (instructions file). Input: {\"name\": \"my-skill\", \"description\": \"What it does\", \"body\": \"Detailed instructions...\"}. Skills are saved to .serana/skills/<name>/SKILL.md and become available to the agent."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name (kebab-case, used as directory name)"
                },
                "description": {
                    "type": "string",
                    "description": "One-line description of what this skill does"
                },
                "body": {
                    "type": "string",
                    "description": "Full instructions/prompt content for the skill"
                }
            },
            "required": ["name", "description", "body"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let body = input
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'body' field"))?;

        // Validate name
        if name.is_empty() || name.contains('/') || name.contains("..") {
            return Err(anyhow::anyhow!(
                "Invalid skill name: must be non-empty and contain no '/' or '..'"
            ));
        }

        let skill_dir = self.workspace.join(".serana").join("skills").join(name);
        let skill_file = skill_dir.join("SKILL.md");

        tokio::fs::create_dir_all(&skill_dir).await?;

        let content = format!(
            "---\ndescription: \"{}\"\n---\n\n{}",
            description.replace('"', "\\\""),
            body,
        );
        tokio::fs::write(&skill_file, &content).await?;

        Ok(json!({
            "status": "created",
            "name": name,
            "path": skill_file.to_string_lossy(),
            "description": description,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_support::tempdir;

    #[test]
    fn parses_frontmatter() {
        let content = "---\ndescription: \"Test skill\"\n---\n\n# Instructions\nDo the thing.";
        let (desc, body) = parse_skill_md(content);
        assert_eq!(desc, "Test skill");
        assert!(body.contains("Instructions"));
        assert!(body.contains("Do the thing."));
    }

    #[test]
    fn parses_no_frontmatter() {
        let content = "# My Skill\nSome instructions here.";
        let (desc, body) = parse_skill_md(content);
        assert_eq!(desc, "My Skill");
        assert_eq!(body, content);
    }

    #[test]
    fn discovers_skills_from_dir() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: A test\n---\n\nTest body",
        )
        .unwrap();

        let mut store = SkillStore::new();
        store.scan_dir(dir.path(), SkillSource::Project);

        assert_eq!(store.len(), 1);
        let skill = store.get("test-skill").unwrap();
        assert_eq!(skill.description, "A test");
        assert_eq!(skill.body, "Test body");
        assert_eq!(skill.source, SkillSource::Project);
    }

    #[test]
    fn render_for_prompt() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: Does X\n---\n\nDo X like this.",
        )
        .unwrap();

        let mut store = SkillStore::new();
        store.scan_dir(dir.path(), SkillSource::User);
        let prompt = store.render_for_prompt().unwrap();
        assert!(prompt.contains("<skill name=\"my-skill\""));
        assert!(prompt.contains("Do X like this."));
    }

    #[tokio::test]
    async fn skill_create_tool() {
        let dir = tempdir().unwrap();
        let tool = SkillCreateTool::new(dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "name": "test-skill",
                "description": "A test skill",
                "body": "Instructions here."
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "created");
        assert_eq!(result["name"], "test-skill");

        let skill_file = dir
            .path()
            .join(".serana")
            .join("skills")
            .join("test-skill")
            .join("SKILL.md");
        assert!(skill_file.exists());
        let content = std::fs::read_to_string(&skill_file).unwrap();
        assert!(content.contains("A test skill"));
        assert!(content.contains("Instructions here."));
    }
}
