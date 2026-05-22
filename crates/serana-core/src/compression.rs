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
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 128_000,
            protect_last_n: 10,
            thresholds: CompressionThresholds::default(),
        }
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
