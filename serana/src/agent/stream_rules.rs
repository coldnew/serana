//! Time-Traveling Stream Rules (TTSR).
//!
//! Monitors the LLM streaming output for regex patterns. When a pattern
//! matches mid-stream, the request is aborted, the rule is injected as
//! a system reminder, and the stream retries from the same point.
//!
//! Enhanced with:
//! - Interrupt modes: "always" (abort+retry) or "never" (deferred injection)
//! - Scope filtering: match against text, thinking, or tool output
//! - Repeat policy: "once" or "after-gap" (N turns between re-triggers)
//! - Context modes: "discard" (remove partial output) or "keep" (append reminder)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Interrupt mode for a stream rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptMode {
    /// Abort the stream and retry with injection (default).
    Always,
    /// Defer injection until after the current tool call completes.
    Never,
}

impl Default for InterruptMode {
    fn default() -> Self {
        Self::Always
    }
}

/// Which stream sources a rule applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    /// Match against assistant text output (default).
    Text,
    /// Match against thinking/reasoning output.
    Thinking,
    /// Match against tool call output.
    Tool,
}

impl Default for RuleScope {
    fn default() -> Self {
        Self::Text
    }
}

/// Repeat policy for a stream rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatPolicy {
    /// Only trigger once per session (default).
    Once,
    /// Re-trigger after N turns since last injection.
    AfterGap(usize),
}

impl Default for RepeatPolicy {
    fn default() -> Self {
        Self::Once
    }
}

/// Context mode — what to do with partial output when a rule triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
    /// Discard partial output and retry from scratch (default).
    Discard,
    /// Keep partial output and append the injection as a reminder.
    Keep,
}

impl Default for ContextMode {
    fn default() -> Self {
        Self::Discard
    }
}

/// A stream rule that triggers when output matches a regex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRule {
    /// Rule name for logging.
    pub name: String,
    /// Regex pattern to match against accumulated output.
    pub pattern: String,
    /// System message to inject when triggered.
    pub injection: String,
    /// When to interrupt the stream. Default: Always.
    #[serde(default)]
    pub interrupt: InterruptMode,
    /// Which stream source to monitor. Default: Text.
    #[serde(default)]
    pub scope: RuleScope,
    /// Repeat policy. Default: Once.
    #[serde(default)]
    pub repeat: RepeatPolicy,
    /// What to do with partial output on trigger. Default: Discard.
    #[serde(default)]
    pub context: ContextMode,
    /// Optional tool name filter (only match when this tool is active).
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Optional path glob filter (only match when file path matches).
    #[serde(default)]
    pub path_glob: Option<String>,
}

/// Result of checking streaming output against rules.
#[derive(Debug, Clone)]
pub enum StreamRuleMatch {
    /// No rule matched.
    None,
    /// A rule matched with "always" interrupt — abort and retry.
    Interrupt {
        name: String,
        injection: String,
        context: ContextMode,
    },
    /// A rule matched with "never" interrupt — defer injection.
    Deferred { name: String, injection: String },
}

/// TTSR engine that monitors streaming output against rules.
pub struct StreamRuleEngine {
    rules: Vec<StreamRule>,
    /// Names of rules already triggered this session, with turn count since last trigger.
    triggered: Vec<(String, usize)>,
    /// Compiled regexes cached alongside rules.
    compiled: Vec<(StreamRule, regex::Regex)>,
    /// Current turn counter for gap-based repeat policies.
    turn_count: usize,
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
            triggered: Vec::new(),
            compiled,
            turn_count: 0,
        }
    }

    /// Create an engine by loading rules from disk.
    pub async fn from_disk() -> Self {
        let rules = load_stream_rules().await;
        Self::new(rules)
    }

    /// Check accumulated output against all rules. Returns the match result.
    pub fn check(&self, output: &str) -> StreamRuleMatch {
        self.check_with_scope(output, &RuleScope::Text, None)
    }

    /// Check with explicit scope and optional tool context.
    pub fn check_with_scope(
        &self,
        output: &str,
        scope: &RuleScope,
        tool_name: Option<&str>,
    ) -> StreamRuleMatch {
        for (rule, re) in &self.compiled {
            // Scope filter
            if rule.scope != *scope {
                continue;
            }

            // Tool name filter
            if let Some(ref filter_tool) = rule.tool_name {
                match tool_name {
                    Some(name) if name == filter_tool => {}
                    _ => continue,
                }
            }

            // Check if rule should trigger based on repeat policy
            if !self.should_trigger(rule) {
                continue;
            }

            if re.is_match(output) {
                return match rule.interrupt {
                    InterruptMode::Always => StreamRuleMatch::Interrupt {
                        name: rule.name.clone(),
                        injection: rule.injection.clone(),
                        context: rule.context,
                    },
                    InterruptMode::Never => StreamRuleMatch::Deferred {
                        name: rule.name.clone(),
                        injection: rule.injection.clone(),
                    },
                };
            }
        }
        StreamRuleMatch::None
    }

    /// Check if a rule should trigger based on its repeat policy.
    fn should_trigger(&self, rule: &StreamRule) -> bool {
        match &rule.repeat {
            RepeatPolicy::Once => !self.triggered.iter().any(|(name, _)| name == &rule.name),
            RepeatPolicy::AfterGap(gap) => {
                match self.triggered.iter().find(|(name, _)| name == &rule.name) {
                    Some((_, last_turn)) => self.turn_count - last_turn >= *gap,
                    None => true,
                }
            }
        }
    }

    /// Mark a rule as triggered.
    pub fn mark_triggered(&mut self, name: &str) {
        if let Some(entry) = self.triggered.iter_mut().find(|(n, _)| n == name) {
            entry.1 = self.turn_count;
        } else {
            self.triggered.push((name.to_string(), self.turn_count));
        }
    }

    /// Get the injection text for a rule by name.
    pub fn get_injection(&self, name: &str) -> Option<&str> {
        self.rules
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.injection.as_str())
    }

    /// Advance the turn counter (call once per completed turn).
    pub fn advance_turn(&mut self) {
        self.turn_count += 1;
    }

    /// Reset all state (e.g., on new session).
    pub fn reset(&mut self) {
        self.triggered.clear();
        self.turn_count = 0;
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
            interrupt: InterruptMode::default(),
            scope: RuleScope::default(),
            repeat: RepeatPolicy::default(),
            context: ContextMode::default(),
            tool_name: None,
            path_glob: None,
        }
    }

    #[test]
    fn detects_matching_output() {
        let engine = StreamRuleEngine::new(vec![test_rule(
            "no-unwrap",
            r"\.unwrap\(\)",
            "Don't use .unwrap() in library code",
        )]);

        assert!(matches!(
            engine.check("let x = foo.unwrap();"),
            StreamRuleMatch::Interrupt { .. }
        ));
        assert!(matches!(
            engine.check("let x = foo?;"),
            StreamRuleMatch::None
        ));
    }

    #[test]
    fn skips_already_triggered_once_rules() {
        let mut engine = StreamRuleEngine::new(vec![test_rule(
            "no-unwrap",
            r"\.unwrap\(\)",
            "Don't use .unwrap()",
        )]);

        assert!(matches!(
            engine.check("foo.unwrap()"),
            StreamRuleMatch::Interrupt { .. }
        ));
        engine.mark_triggered("no-unwrap");
        assert!(matches!(
            engine.check("bar.unwrap()"),
            StreamRuleMatch::None
        ));
    }

    #[test]
    fn after_gap_repeat_policy() {
        let rule = StreamRule {
            repeat: RepeatPolicy::AfterGap(3),
            ..test_rule("no-unwrap", r"\.unwrap\(\)", "Don't unwrap")
        };
        let mut engine = StreamRuleEngine::new(vec![rule]);

        // First trigger works
        assert!(matches!(
            engine.check("foo.unwrap()"),
            StreamRuleMatch::Interrupt { .. }
        ));
        engine.mark_triggered("no-unwrap");

        // Not enough gap yet
        engine.advance_turn();
        assert!(matches!(
            engine.check("bar.unwrap()"),
            StreamRuleMatch::None
        ));
        engine.advance_turn();
        assert!(matches!(
            engine.check("baz.unwrap()"),
            StreamRuleMatch::None
        ));

        // After 3 turns, can trigger again
        engine.advance_turn();
        assert!(matches!(
            engine.check("qux.unwrap()"),
            StreamRuleMatch::Interrupt { .. }
        ));
    }

    #[test]
    fn never_interrupt_is_deferred() {
        let rule = StreamRule {
            interrupt: InterruptMode::Never,
            ..test_rule("soft-warning", r"TODO", "Remove TODOs before committing")
        };
        let engine = StreamRuleEngine::new(vec![rule]);

        match engine.check("// TODO: fix this") {
            StreamRuleMatch::Deferred { name, .. } => assert_eq!(name, "soft-warning"),
            _ => panic!("Expected Deferred match"),
        }
    }

    #[test]
    fn scope_filtering() {
        let rule = StreamRule {
            scope: RuleScope::Thinking,
            ..test_rule("no-reason", r"I think", "Be more decisive")
        };
        let engine = StreamRuleEngine::new(vec![rule]);

        // Should not match text scope
        assert!(matches!(
            engine.check_with_scope("I think", &RuleScope::Text, None),
            StreamRuleMatch::None
        ));

        // Should match thinking scope
        assert!(matches!(
            engine.check_with_scope("I think", &RuleScope::Thinking, None),
            StreamRuleMatch::Interrupt { .. }
        ));
    }

    #[test]
    fn tool_name_filter() {
        let rule = StreamRule {
            tool_name: Some("shell".to_string()),
            ..test_rule("no-sudo", r"sudo", "Avoid sudo in commands")
        };
        let engine = StreamRuleEngine::new(vec![rule]);

        // Should not match when different tool
        assert!(matches!(
            engine.check_with_scope("sudo apt install", &RuleScope::Text, Some("read_file")),
            StreamRuleMatch::None
        ));

        // Should match when correct tool
        assert!(matches!(
            engine.check_with_scope("sudo apt install", &RuleScope::Text, Some("shell")),
            StreamRuleMatch::Interrupt { .. }
        ));
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
    fn reset_clears_state() {
        let mut engine = StreamRuleEngine::new(vec![test_rule("rule1", r"pattern1", "injection1")]);

        engine.mark_triggered("rule1");
        assert!(matches!(engine.check("pattern1"), StreamRuleMatch::None));

        engine.reset();
        assert!(matches!(
            engine.check("pattern1"),
            StreamRuleMatch::Interrupt { .. }
        ));
    }

    #[test]
    fn deserialize_from_toml() {
        let toml = r#"
            name = "no-sudo"
            pattern = "sudo"
            injection = "Avoid sudo"
            interrupt = "never"
            scope = "tool"
            repeat = { after_gap = 5 }
            context = "keep"
            tool_name = "shell"
        "#;
        let rule: StreamRule = toml::from_str(toml).unwrap();
        assert_eq!(rule.name, "no-sudo");
        assert_eq!(rule.interrupt, InterruptMode::Never);
        assert_eq!(rule.scope, RuleScope::Tool);
        assert_eq!(rule.repeat, RepeatPolicy::AfterGap(5));
        assert_eq!(rule.context, ContextMode::Keep);
        assert_eq!(rule.tool_name.as_deref(), Some("shell"));
    }
}
