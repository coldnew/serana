//! Rules and skills system for Serana.
//!
//! Rules: regex patterns that trigger when the LLM output matches.
//! Skills: markdown files loaded into the system prompt context.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use crate::core::{Result, Tool};

/// A rule that triggers when the LLM output matches a regex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub pattern: String,
    pub injection: String,
}

/// Load rules from ~/.serana/rules/*.toml
pub async fn load_rules() -> Vec<Rule> {
    let rules_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".serana")
        .join("rules");

    let mut rules = Vec::new();

    if let Ok(mut entries) = fs::read_dir(&rules_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(rule) = toml::from_str::<Rule>(&content) {
                        rules.push(rule);
                    }
                }
            }
        }
    }

    // Also check project-local .serana/rules/
    let project_rules_dir = PathBuf::from(".serana").join("rules");
    if let Ok(mut entries) = fs::read_dir(&project_rules_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(rule) = toml::from_str::<Rule>(&content) {
                        rules.push(rule);
                    }
                }
            }
        }
    }

    rules
}

/// Load skills from ~/.serana/skills/*.md
pub async fn load_skills() -> Vec<String> {
    let skills_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".serana")
        .join("skills");

    let mut skills = Vec::new();

    if let Ok(mut entries) = fs::read_dir(&skills_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    skills.push(content);
                }
            }
        }
    }

    skills
}

/// Check if LLM output matches any rule pattern.
pub fn check_rules<'a>(output: &str, rules: &'a [Rule]) -> Option<&'a Rule> {
    for rule in rules {
        if let Ok(re) = regex::Regex::new(&rule.pattern) {
            if re.is_match(output) {
                return Some(rule);
            }
        }
    }
    None
}

/// Tool to list loaded rules and skills.
pub struct RulesInfoTool;

#[async_trait]
impl Tool for RulesInfoTool {
    fn name(&self) -> &'static str {
        "rules_info"
    }

    fn description(&self) -> &'static str {
        "Show loaded rules and skills. Input: {}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let rules = load_rules().await;
        let skills = load_skills().await;

        Ok(json!({
            "rules": rules.iter().map(|r| json!({
                "name": r.name,
                "pattern": r.pattern,
            })).collect::<Vec<_>>(),
            "skills_count": skills.len(),
        }))
    }
}
