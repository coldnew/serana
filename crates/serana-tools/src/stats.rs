//! Stats and observability for token usage and costs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use serana_core::{Result, Tool};

/// Per-session usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub tool_calls: u64,
    pub llm_calls: u64,
    pub errors: u64,
    pub start_time: Option<String>,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            start_time: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }

    pub fn record_tokens(&mut self, input: u64, output: u64) {
        self.tokens_in += input;
        self.tokens_out += output;
        self.llm_calls += 1;
    }

    pub fn record_tool_call(&mut self) {
        self.tool_calls += 1;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}

/// Tool to show current session statistics.
pub struct StatsTool {
    stats: std::sync::Mutex<SessionStats>,
}

impl StatsTool {
    pub fn new() -> Self {
        Self {
            stats: std::sync::Mutex::new(SessionStats::new()),
        }
    }

    pub fn record_tokens(&self, input: u64, output: u64) {
        if let Ok(mut s) = self.stats.lock() {
            s.record_tokens(input, output);
        }
    }

    pub fn record_tool_call(&self) {
        if let Ok(mut s) = self.stats.lock() {
            s.record_tool_call();
        }
    }

    pub fn record_error(&self) {
        if let Ok(mut s) = self.stats.lock() {
            s.record_error();
        }
    }
}

impl Default for StatsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for StatsTool {
    fn name(&self) -> &'static str {
        "stats"
    }

    fn description(&self) -> &'static str {
        "Show current session statistics (tokens, tool calls, errors). Input: {}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let stats = self.stats.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(json!({
            "tokens_in": stats.tokens_in,
            "tokens_out": stats.tokens_out,
            "total_tokens": stats.tokens_in + stats.tokens_out,
            "tool_calls": stats.tool_calls,
            "llm_calls": stats.llm_calls,
            "errors": stats.errors,
            "start_time": stats.start_time,
        }))
    }
}
