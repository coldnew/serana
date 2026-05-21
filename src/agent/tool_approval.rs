//! Tool approval system for controlling dangerous operations.
//!
//! Provides approval modes and risk classification for tool calls.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Tool approval mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalMode {
    /// Automatically approve all tool calls
    Auto,
    /// Require user approval for every tool call
    Interactive,
    /// Automatically approve whitelisted tools, ask for others
    Whitelist(HashSet<String>),
    /// Automatically approve safe tools, ask for dangerous ones
    Smart,
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::Smart
    }
}

/// Risk level for tool operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Safe operations (read-only, no side effects)
    Safe,
    /// Low-risk operations (local writes, non-destructive)
    Low,
    /// Medium-risk operations (file modifications, local commands)
    Medium,
    /// High-risk operations (deletions, network requests, system changes)
    High,
}

/// Tool approval decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Approve this tool call
    Approve,
    /// Deny this tool call
    Deny,
    /// Approve and add to whitelist
    ApproveAlways,
    /// Deny and add to blacklist
    DenyAlways,
}

/// Tool approval system.
#[derive(Debug, Clone)]
pub struct ToolApproval {
    mode: ApprovalMode,
    whitelist: HashSet<String>,
    blacklist: HashSet<String>,
}

impl ToolApproval {
    /// Create a new tool approval system with the given mode.
    pub fn new(mode: ApprovalMode) -> Self {
        let whitelist = match &mode {
            ApprovalMode::Whitelist(list) => list.clone(),
            _ => HashSet::new(),
        };

        Self {
            mode,
            whitelist,
            blacklist: HashSet::new(),
        }
    }

    /// Create with default smart mode.
    pub fn smart() -> Self {
        Self::new(ApprovalMode::Smart)
    }

    /// Create with auto-approve mode.
    pub fn auto() -> Self {
        Self::new(ApprovalMode::Auto)
    }

    /// Create with interactive mode.
    pub fn interactive() -> Self {
        Self::new(ApprovalMode::Interactive)
    }

    pub fn requires_approval(&self, tool_name: &str) -> bool {
        // Check blacklist first
        if self.blacklist.contains(tool_name) {
            return false;
        }

        // Whitelist overrides everything
        if self.whitelist.contains(tool_name) {
            return false;
        }

        match &self.mode {
            ApprovalMode::Auto => false,
            ApprovalMode::Interactive => true,
            ApprovalMode::Whitelist(_) => true,
            ApprovalMode::Smart => {
                let risk = Self::classify_risk(tool_name);
                risk >= RiskLevel::High
            }
        }
    }

    /// Classify the risk level of a tool.
    pub fn classify_risk(tool_name: &str) -> RiskLevel {
        match tool_name {
            // Safe: read-only operations
            "read_file" | "list_files" | "search_code" | "get_definition" | "find_references"
            | "get_hover" | "get_diagnostics" => RiskLevel::Safe,

            // Low: local non-destructive writes
            "write_file" | "create_file" | "append_file" => RiskLevel::Low,

            // Medium: modifications and local commands
            "edit_file" | "rename_file" | "run_command" | "git_commit" => RiskLevel::Medium,

            // High: destructive or network operations
            "delete_file" | "delete_directory" | "git_push" | "http_request"
            | "install_package" | "system_command" => RiskLevel::High,

            // Unknown tools default to high risk
            _ => RiskLevel::High,
        }
    }

    /// Apply an approval decision.
    pub fn apply_decision(&mut self, tool_name: &str, decision: ApprovalDecision) {
        match decision {
            ApprovalDecision::Approve => {
                // One-time approval, no state change
            }
            ApprovalDecision::Deny => {
                // One-time denial, no state change
            }
            ApprovalDecision::ApproveAlways => {
                self.whitelist.insert(tool_name.to_string());
                self.blacklist.remove(tool_name);
            }
            ApprovalDecision::DenyAlways => {
                self.blacklist.insert(tool_name.to_string());
                self.whitelist.remove(tool_name);
            }
        }
    }

    /// Get the current approval mode.
    pub fn mode(&self) -> &ApprovalMode {
        &self.mode
    }

    /// Get the whitelist.
    pub fn whitelist(&self) -> &HashSet<String> {
        &self.whitelist
    }

    /// Get the blacklist.
    pub fn blacklist(&self) -> &HashSet<String> {
        &self.blacklist
    }

    /// Check if a tool is whitelisted.
    pub fn is_whitelisted(&self, tool_name: &str) -> bool {
        self.whitelist.contains(tool_name)
    }

    /// Check if a tool is blacklisted.
    pub fn is_blacklisted(&self, tool_name: &str) -> bool {
        self.blacklist.contains(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_mode_approves_all() {
        let approval = ToolApproval::auto();
        assert!(!approval.requires_approval("delete_file"));
        assert!(!approval.requires_approval("read_file"));
    }

    #[test]
    fn interactive_mode_requires_all() {
        let approval = ToolApproval::interactive();
        assert!(approval.requires_approval("delete_file"));
        assert!(approval.requires_approval("read_file"));
    }

    #[test]
    fn smart_mode_requires_high_risk() {
        let approval = ToolApproval::smart();
        assert!(approval.requires_approval("delete_file"));
        assert!(approval.requires_approval("git_push"));
        assert!(!approval.requires_approval("read_file"));
        assert!(!approval.requires_approval("write_file"));
    }

    #[test]
    fn whitelist_mode_checks_list() {
        let mut whitelist = HashSet::new();
        whitelist.insert("read_file".to_string());
        whitelist.insert("write_file".to_string());

        let approval = ToolApproval::new(ApprovalMode::Whitelist(whitelist));
        assert!(!approval.requires_approval("read_file"));
        assert!(!approval.requires_approval("write_file"));
        assert!(approval.requires_approval("delete_file"));
    }

    #[test]
    fn classifies_risk_correctly() {
        assert_eq!(ToolApproval::classify_risk("read_file"), RiskLevel::Safe);
        assert_eq!(ToolApproval::classify_risk("write_file"), RiskLevel::Low);
        assert_eq!(ToolApproval::classify_risk("edit_file"), RiskLevel::Medium);
        assert_eq!(ToolApproval::classify_risk("delete_file"), RiskLevel::High);
        assert_eq!(ToolApproval::classify_risk("unknown_tool"), RiskLevel::High);
    }

    #[test]
    fn approve_always_adds_to_whitelist() {
        let mut approval = ToolApproval::smart();
        approval.apply_decision("delete_file", ApprovalDecision::ApproveAlways);

        assert!(approval.is_whitelisted("delete_file"));
        assert!(!approval.requires_approval("delete_file"));
    }

    #[test]
    fn deny_always_adds_to_blacklist() {
        let mut approval = ToolApproval::auto();
        approval.apply_decision("delete_file", ApprovalDecision::DenyAlways);

        assert!(approval.is_blacklisted("delete_file"));
    }

    #[test]
    fn blacklist_overrides_whitelist() {
        let mut approval = ToolApproval::smart();
        approval.apply_decision("delete_file", ApprovalDecision::ApproveAlways);
        approval.apply_decision("delete_file", ApprovalDecision::DenyAlways);

        assert!(!approval.is_whitelisted("delete_file"));
        assert!(approval.is_blacklisted("delete_file"));
    }
}
