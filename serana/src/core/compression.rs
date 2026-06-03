/// Compression trigger thresholds as percentages of max context.
#[derive(Debug, Clone, Copy)]
pub struct CompressionThresholds {
    /// Preflight check threshold (default 50%)
    pub preflight: f32,
    /// Gateway threshold requiring immediate compression (default 85%)
    pub gateway: f32,
}

impl Default for CompressionThresholds {
    fn default() -> Self {
        Self {
            preflight: 0.50,
            gateway: 0.85,
        }
    }
}

/// Context compression configuration.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum context window size in tokens
    pub max_tokens: usize,
    /// Number of recent messages to protect from compression
    pub protect_last_n: usize,
    /// Compression thresholds
    pub thresholds: CompressionThresholds,
    pub budget_tokens: usize,
    pub target_tokens: usize,
    pub keep_first: usize,
    pub keep_recent: usize,
    pub max_messages: usize,
    pub message_limit_target_pct: u8,
    pub tool_output_max_lines: usize,
    pub microcompact_keep_tokens: usize,
    pub oversize_abs_tokens: usize,
    pub oversize_budget_ratio: f64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let max_tokens = 128_000;
        Self {
            max_tokens,
            protect_last_n: 10,
            thresholds: CompressionThresholds::default(),
            budget_tokens: max_tokens,
            target_tokens: max_tokens * 70 / 100,
            keep_first: 1,
            keep_recent: 10,
            max_messages: 0,
            message_limit_target_pct: 80,
            tool_output_max_lines: 80,
            microcompact_keep_tokens: 8_000,
            oversize_abs_tokens: 4_000,
            oversize_budget_ratio: 0.10,
        }
    }
}

impl CompressionConfig {
    pub fn compact_trigger(&self) -> usize {
        (self.budget_tokens as f32 * self.thresholds.gateway) as usize
    }

    pub fn compact_target(&self) -> usize {
        self.target_tokens
    }
}

/// Compression decision based on current token usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionDecision {
    /// No compression needed
    None,
    /// Compression recommended but not required
    Preflight,
    /// Compression required before next API call
    Gateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionMethod {
    LifecycleReclaimed,
    AgeCleared,
    OversizeCapped,
    HeadTail,
    TurnCollapsed,
    MessagesEvicted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionAction {
    pub index: usize,
    pub tool_name: String,
    pub method: CompactionMethod,
    pub before_tokens: usize,
    pub after_tokens: usize,
    pub end_index: Option<usize>,
    pub related_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolTokenDetail {
    pub tool_name: String,
    pub tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionStats {
    pub level: u8,
    pub before_message_count: usize,
    pub after_message_count: usize,
    pub before_estimated_tokens: usize,
    pub after_estimated_tokens: usize,
    pub tool_outputs_truncated: usize,
    pub turns_summarized: usize,
    pub messages_dropped: usize,
    pub current_run_cleared: usize,
    pub oversize_capped: usize,
    pub age_cleared: usize,
    pub before_tool_details: Vec<ToolTokenDetail>,
    pub after_tool_details: Vec<ToolTokenDetail>,
    pub actions: Vec<CompactionAction>,
}

#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub messages: Vec<super::Message>,
    pub stats: CompactionStats,
}
