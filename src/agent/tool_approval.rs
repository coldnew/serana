//! Tool approval system for dangerous operations.
//!
//! Provides user confirmation workflow for destructive or high-risk tool calls.

use serde_json::Value;

/// Risk level of a tool operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Safe operations (read-only)
    Safe,
    /// Moderate risk (creates new files)
    Moderate,
    /// High risk (modifies or deletes files)
    High,
    /// Critical (destructive, affects multiple files or external systems)
    Critical,
}

/// Approval decision from user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Approved for this execution
    Approved,
    /// Rejected
    Rejected,
    /// Approved for all similar operations in this session
    ApprovedAll,
}

/// Tool approval policy.
#[derive(Debug, Clone)]
pub struct ApprovalPolicy {
    /// Require approval for high-risk operations
    pub require_high_approval: bool,
    /// Require approval for moderate-risk operations
    pub require_moderate_approval: bool,
    /// Auto-approve safe operations
    pub auto_approve_safe: bool,
    /// Paths that always require approval (e.g., production configs)
    pub protected_paths: Vec<String>,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self {
            require_high_approval: true,
            require_moderate_approval: false,
            auto_approve_safe: true,
            protected_paths: vec![
                ".env".to_string(),
                "config.prod".to_string(),
                "credentials".to_string(),
            ],
        }
    }
}

impl ApprovalPolicy {
    /// Create a permissive policy (auto-approve everything).
    pub fn permissive() -> Self {
        Self {
            require_high_approval: false,
            require_moderate_approval: false,
            auto_approve_safe: true,
            protected_paths: vec![],
        }
    }

    /// Create a strict policy (require approval for all modifications).
    pub fn strict() -> Self {
        Self {
            require_high_approval: true,
            require_moderate_approval: true,
            auto_approve_safe: true,
            protected_paths: vec![],
        }
    }

    /// Determine risk level for a tool call.
    pub fn assess_risk(&self, tool_name: &str, arguments: &Value) -> RiskLevel {
        match tool_name {
            "read_file" | "list_files" | "search" | "grep" => RiskLevel::Safe,
            "write_file" => {
                if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                    if self.is_protected_path(path) {
                        return RiskLevel::Critical;
                    }
                    // Writing to existing file is high risk
                    if std::path::Path::new(path).exists() {
                        return RiskLevel::High;
                    }
                    // New file is moderate
                    return RiskLevel::Moderate;
                }
                RiskLevel::High
            }
            "edit_file" => {
                if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                    if self.is_protected_path(path) {
                        return RiskLevel::Critical;
                    }
                }
                RiskLevel::High
            }
            "delete_file" | "rm" | "execute_shell" => RiskLevel::Critical,
            _ => RiskLevel::High, // Unknown tools are high risk
        }
    }

    /// Check if a path is protected.
    pub fn is_protected_path(&self, path: &str) -> bool {
        self.protected_paths.iter().any(|p| path.contains(p))
    }

    /// Check if approval is required for a tool call.
    pub fn requires_approval(&self, tool_name: &str, arguments: &Value) -> bool {
        let risk = self.assess_risk(tool_name, arguments);
        match risk {
            RiskLevel::Safe => !self.auto_approve_safe,
            RiskLevel::Moderate => self.require_moderate_approval,
            RiskLevel::High => self.require_high_approval,
            RiskLevel::Critical => true,
        }
    }
}

/// Approval callback trait for UI integration.
pub trait ApprovalCallback: Send + Sync {
    /// Request approval for a tool call.
    /// Returns the user's decision.
    fn request_approval(&self, tool_name: &str, arguments: &Value, risk: RiskLevel) -> ApprovalDecision;
}

/// Default approval callback that auto-approves everything.
pub struct AutoApprove;

impl ApprovalCallback for AutoApprove {
    fn request_approval(&self, _tool_name: &str, _arguments: &Value, _risk: RiskLevel) -> ApprovalDecision {
        ApprovalDecision::ApprovedAll
    }
}

/// In-memory approval state for session-level decisions.
#[derive(Debug, Clone, Default)]
pub struct ApprovalState {
    /// Tools that have been approved for all similar operations
    approved_all: Vec<String>,
    /// Policy configuration
    policy: ApprovalPolicy,
}

impl ApprovalState {
    pub fn new(policy: ApprovalPolicy) -> Self {
        Self {
            approved_all: Vec::new(),
            policy,
        }
    }

    /// Check if a tool call is already approved (from previous ApprovedAll).
    pub fn is_preapproved(&self, tool_name: &str) -> bool {
        self.approved_all.iter().any(|t| t == tool_name)
    }

    /// Record an ApprovedAll decision.
    pub fn record_approved_all(&mut self, tool_name: &str) {
        if !self.is_preapproved(tool_name) {
            self.approved_all.push(tool_name.to_string());
        }
    }

    /// Check if approval is needed for a tool call.
    pub fn needs_approval(&self, tool_name: &str, arguments: &Value) -> bool {
        if self.is_preapproved(tool_name) {
            return false;
        }
        self.policy.requires_approval(tool_name, arguments)
    }

    /// Get the risk level for a tool call.
    pub fn get_risk_level(&self, tool_name: &str, arguments: &Value) -> RiskLevel {
        self.policy.assess_risk(tool_name, arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn read_file_is_safe() {
        let policy = ApprovalPolicy::default();
        assert_eq!(
            policy.assess_risk("read_file", &json!({"path": "test.txt"})),
            RiskLevel::Safe
        );
    }

    #[test]
    fn write_new_file_is_moderate() {
        let policy = ApprovalPolicy::default();
        assert_eq!(
            policy.assess_risk("write_file", &json!({"path": "/tmp/new_file.txt"})),
            RiskLevel::Moderate
        );
    }

    #[test]
    fn write_existing_file_is_high() {
        let policy = ApprovalPolicy::default();
        // Cargo.toml exists in the workspace
        assert_eq!(
            policy.assess_risk("write_file", &json!({"path": "Cargo.toml"})),
            RiskLevel::High
        );
    }

    #[test]
    fn protected_paths_are_critical() {
        let policy = ApprovalPolicy::default();
        assert_eq!(
            policy.assess_risk("write_file", &json!({"path": ".env"})),
            RiskLevel::Critical
        );
        assert_eq!(
            policy.assess_risk("edit_file", &json!({"path": "config.prod.yaml"})),
            RiskLevel::Critical
        );
    }

    #[test]
    fn policy_requires_approval_correctly() {
        let policy = ApprovalPolicy::default();
        
        // Safe operations don't need approval
        assert!(!policy.requires_approval("read_file", &json!({"path": "test.txt"})));
        
        // High risk needs approval
        assert!(policy.requires_approval("write_file", &json!({"path": "Cargo.toml"})));
        
        // Critical always needs approval
        assert!(policy.requires_approval("write_file", &json!({"path": ".env"})));
    }

    #[test]
    fn permissive_policy_auto_approves() {
        let policy = ApprovalPolicy::permissive();
        assert!(!policy.requires_approval("write_file", &json!({"path": "Cargo.toml"})));
        assert!(!policy.requires_approval("edit_file", &json!({"path": ".env"})));
    }

    fn approval_state_tracks_approved_all() {
        let mut state = ApprovalState::new(ApprovalPolicy::default());

        // Use an existing file for High risk (requires approval by default)
        assert!(state.needs_approval("write_file", &json!({"path": "Cargo.toml"})));

        state.record_approved_all("write_file");
        assert!(!state.needs_approval("write_file", &json!({"path": "other.txt"})));
    }
}
