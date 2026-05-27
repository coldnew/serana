//! Time-Traveling Stream Rules (TTSR).
//!
//! Monitors the LLM streaming output for regex patterns. When a pattern
//! matches mid-stream, the request is aborted, the rule is injected as
//! a system reminder, and the stream retries from the same point.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// A stream rule that triggers when output matches a regex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRule {
    /// Rule name for logging.
    pub name: String,
    /// Regex pattern to match against accumulated output.
    pub pattern: String,
    /// System message to inject when triggered.
    pub injection: String,
}

/// TTSR engine that monitors streaming output against rules.
pub struct StreamRuleEngine {
    rules: Vec<StreamRule>,
    /// Names of rules already injected this session (avoid re-injection).
    injected: HashSet<String>,
    /// Compiled regexes cached alongside rules.
    compiled: Vec<(StreamRule, regex::Regex)>,
}

impl StreamRuleEngine {
    /// Create a new engine from rules.
    pub fn new(rules: Vec<StreamRule>) -> Self {
        let compiled: Vec<_> = rules
            .iter()
            .filter_map(|r| regex::Regex::new(&r.pattern).ok().map(|re| (r.clone(), re)))
            .collect();

        Self {
            rules,
            injected: HashSet::new(),
            compiled,
        }
    }

    /// Create an engine by loading rules from disk.
    pub async fn from_disk() -> Self {
        let rules = load_stream_rules().await;
        Self::new(rules)
    }

    /// Check accumulated output against all rules. Returns the first
    /// rule that matches (if any), unless it was already injected.
    pub fn check(&self, output: &str) -> Option<&StreamRule> {
        for (rule, re) in &self.compiled {
            if self.injected.contains(&rule.name) {
                continue;
            }
            if re.is_match(output) {
                return Some(rule);
            }
        }
        None
    }

    /// Mark a rule as injected to avoid re-injection.
    pub fn mark_injected(&mut self, name: &str) {
        self.injected.insert(name.to_string());
    }

    /// Get the injection text for a rule by name.
    pub fn get_injection(&self, name: &str) -> Option<&str> {
        self.rules
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.injection.as_str())
    }

    /// Reset injected state (e.g., on new session).
    pub fn reset(&mut self) {
        self.injected.clear();
    }

    /// Number of loaded rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Load stream rules from ~/.serana/rules/*.toml and .serana/rules/*.toml.
/// Same directory as the regular rules system, but filters for rules
/// that have the `injection` field.
pub async fn load_stream_rules() -> Vec<StreamRule> {
    let mut rules = Vec::new();

    let home_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".serana")
        .join("rules");

    let project_dir = PathBuf::from(".serana").join("rules");

    for dir in &[home_dir, project_dir] {
        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(rule) = toml::from_str::<StreamRule>(&content) {
                            rules.push(rule);
                        }
                    }
                }
            }
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rule(name: &str, pattern: &str, injection: &str) -> StreamRule {
        StreamRule {
            name: name.to_string(),
            pattern: pattern.to_string(),
            injection: injection.to_string(),
        }
    }

    #[test]
    fn detects_matching_output() {
        let engine = StreamRuleEngine::new(vec![test_rule(
            "no-unwrap",
            r"\.unwrap\(\)",
            "Don't use .unwrap() in library code",
        )]);

        assert!(engine.check("let x = foo.unwrap();").is_some());
        assert!(engine.check("let x = foo?;").is_none());
    }

    #[test]
    fn skips_already_injected_rules() {
        let mut engine = StreamRuleEngine::new(vec![test_rule(
            "no-unwrap",
            r"\.unwrap\(\)",
            "Don't use .unwrap()",
        )]);

        assert!(engine.check("foo.unwrap()").is_some());
        engine.mark_injected("no-unwrap");
        assert!(engine.check("bar.unwrap()").is_none());
    }

    #[test]
    fn get_injection_text() {
        let engine = StreamRuleEngine::new(vec![test_rule(
            "no-leak",
            r"Box::leak",
            "Use Arc instead of Box::leak",
        )]);

        assert_eq!(
            engine.get_injection("no-leak"),
            Some("Use Arc instead of Box::leak")
        );
        assert!(engine.get_injection("unknown").is_none());
    }

    #[test]
    fn reset_clears_injected_state() {
        let mut engine = StreamRuleEngine::new(vec![test_rule("rule1", r"pattern1", "injection1")]);

        engine.mark_injected("rule1");
        assert!(engine.check("pattern1").is_none());

        engine.reset();
        assert!(engine.check("pattern1").is_some());
    }
}
