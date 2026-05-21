//! Callback surfaces for Hermes-style agent.
//!
//! Enables real-time progress updates for CLI, gateway, and other integrations.

use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool execution progress callback.
/// Called before and after each tool execution.
pub type ToolProgressCallback = Arc<dyn Fn(&str, &str, bool) + Send + Sync>;

/// Thinking state callback.
/// Called when model starts/stops thinking.
pub type ThinkingCallback = Arc<dyn Fn(bool) + Send + Sync>;

/// Reasoning content callback.
/// Called when model returns reasoning/thinking content.
pub type ReasoningCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Streaming delta callback.
/// Called for each streaming token.
pub type StreamDeltaCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Status change callback.
/// Called when agent state changes.
pub type StatusCallback = Arc<dyn Fn(AgentStatus) + Send + Sync>;

/// Agent status states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Thinking,
    ExecutingTool,
    WaitingForInput,
    Compressing,
    Error,
}

/// Collection of all callback surfaces.
#[derive(Default, Clone)]
pub struct AgentCallbacks {
    pub tool_progress: Option<ToolProgressCallback>,
    pub thinking: Option<ThinkingCallback>,
    pub reasoning: Option<ReasoningCallback>,
    pub stream_delta: Option<StreamDeltaCallback>,
    pub status: Option<StatusCallback>,
}

impl std::fmt::Debug for AgentCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCallbacks")
            .field("has_tool_progress", &self.tool_progress.is_some())
            .field("has_thinking", &self.thinking.is_some())
            .field("has_reasoning", &self.reasoning.is_some())
            .field("has_stream_delta", &self.stream_delta.is_some())
            .field("has_status", &self.status.is_some())
            .finish()
    }
}

impl AgentCallbacks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tool progress callback.
    pub fn with_tool_progress(mut self, cb: ToolProgressCallback) -> Self {
        self.tool_progress = Some(cb);
        self
    }

    /// Set thinking callback.
    pub fn with_thinking(mut self, cb: ThinkingCallback) -> Self {
        self.thinking = Some(cb);
        self
    }

    /// Set reasoning callback.
    pub fn with_reasoning(mut self, cb: ReasoningCallback) -> Self {
        self.reasoning = Some(cb);
        self
    }

    /// Set stream delta callback.
    pub fn with_stream_delta(mut self, cb: StreamDeltaCallback) -> Self {
        self.stream_delta = Some(cb);
        self
    }

    /// Set status callback.
    pub fn with_status(mut self, cb: StatusCallback) -> Self {
        self.status = Some(cb);
        self
    }

    /// Fire tool progress event.
    pub fn fire_tool_progress(&self, tool_name: &str, args: &str, is_complete: bool) {
        if let Some(cb) = &self.tool_progress {
            cb(tool_name, args, is_complete);
        }
    }

    /// Fire thinking event.
    pub fn fire_thinking(&self, is_thinking: bool) {
        if let Some(cb) = &self.thinking {
            cb(is_thinking);
        }
    }

    /// Fire reasoning event.
    pub fn fire_reasoning(&self, content: &str) {
        if let Some(cb) = &self.reasoning {
            cb(content);
        }
    }

    /// Fire stream delta event.
    pub fn fire_stream_delta(&self, delta: &str) {
        if let Some(cb) = &self.stream_delta {
            cb(delta);
        }
    }

    /// Fire status event.
    pub fn fire_status(&self, status: AgentStatus) {
        if let Some(cb) = &self.status {
            cb(status);
        }
    }
}

/// Thread-safe callback state for use across async boundaries.
#[derive(Debug, Clone)]
pub struct CallbackState {
    callbacks: Arc<RwLock<AgentCallbacks>>,
}

impl CallbackState {
    pub fn new(callbacks: AgentCallbacks) -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(callbacks)),
        }
    }

    pub fn empty() -> Self {
        Self::new(AgentCallbacks::default())
    }

    /// Update callbacks.
    pub async fn set_callbacks(&self, callbacks: AgentCallbacks) {
        *self.callbacks.write().await = callbacks;
    }

    /// Fire tool progress event.
    pub async fn fire_tool_progress(&self, tool_name: &str, args: &str, is_complete: bool) {
        let cb = self.callbacks.read().await;
        cb.fire_tool_progress(tool_name, args, is_complete);
    }

    /// Fire thinking event.
    pub async fn fire_thinking(&self, is_thinking: bool) {
        let cb = self.callbacks.read().await;
        cb.fire_thinking(is_thinking);
    }

    /// Fire status event.
    pub async fn fire_status(&self, status: AgentStatus) {
        let cb = self.callbacks.read().await;
        cb.fire_status(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_callbacks() {
        let call_count = Arc::new(AtomicU32::new(0));
        let count_clone = call_count.clone();

        let callbacks = AgentCallbacks::new()
            .with_thinking(Arc::new(move |_| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            }));

        callbacks.fire_thinking(true);
        callbacks.fire_thinking(false);

        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_status_enum() {
        let status = AgentStatus::Thinking;
        assert_eq!(status, AgentStatus::Thinking);
        assert_ne!(status, AgentStatus::Idle);
    }
}
