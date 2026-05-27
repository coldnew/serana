use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

/// Manages conversation checkpoints for the agent engine.
///
/// When the agent uses the `checkpoint` tool, the current message index is saved.
/// When `rewind` is used, messages are truncated back to the checkpoint point,
/// discarding exploratory context.
pub struct CheckpointManager {
    checkpoints: Mutex<HashMap<String, usize>>,
}

impl CheckpointManager {
    pub fn new() -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
        }
    }

    /// Record a checkpoint at the given message index.
    pub fn save(&self, label: &str, message_index: usize) {
        let mut checkpoints = self.checkpoints.lock().unwrap();
        checkpoints.insert(label.to_string(), message_index);
    }

    /// Find the message index to rewind to.
    /// If label is given, finds that specific checkpoint.
    /// If no label, finds the most recent checkpoint.
    /// Returns None if no matching checkpoint exists.
    pub fn find_rewind_target(&self, label: Option<&str>) -> Option<usize> {
        let checkpoints = self.checkpoints.lock().unwrap();
        if let Some(label) = label {
            checkpoints.get(label).copied()
        } else {
            checkpoints.values().max().copied()
        }
    }

    /// Remove all checkpoints at or after the given index (used after rewind).
    pub fn clear_after(&self, index: usize) {
        let mut checkpoints = self.checkpoints.lock().unwrap();
        checkpoints.retain(|_, v| *v < index);
    }

    /// Check if a tool result indicates a checkpoint action.
    pub fn is_checkpoint_signal(result: &Value) -> Option<&str> {
        if result.get("checkpoint").and_then(|v| v.as_bool()) == Some(true) {
            result.get("label").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Check if a tool result indicates a rewind action.
    pub fn is_rewind_signal(result: &Value) -> Option<Option<&str>> {
        if result.get("rewind").and_then(|v| v.as_bool()) == Some(true) {
            Some(result.get("label").and_then(|v| v.as_str()))
        } else {
            None
        }
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_save_and_find() {
        let mgr = CheckpointManager::new();
        mgr.save("before refactor", 10);
        mgr.save("after cleanup", 20);

        assert_eq!(mgr.find_rewind_target(Some("before refactor")), Some(10));
        assert_eq!(mgr.find_rewind_target(Some("after cleanup")), Some(20));
        assert_eq!(mgr.find_rewind_target(None), Some(20)); // latest
        assert_eq!(mgr.find_rewind_target(Some("nonexistent")), None);
    }

    #[test]
    fn test_clear_after() {
        let mgr = CheckpointManager::new();
        mgr.save("a", 5);
        mgr.save("b", 10);
        mgr.save("c", 15);

        mgr.clear_after(10);
        assert_eq!(mgr.find_rewind_target(Some("a")), Some(5));
        assert_eq!(mgr.find_rewind_target(Some("b")), None);
        assert_eq!(mgr.find_rewind_target(Some("c")), None);
    }

    #[test]
    fn test_checkpoint_signal_detection() {
        let result = json!({"checkpoint": true, "label": "test"});
        assert_eq!(CheckpointManager::is_checkpoint_signal(&result), Some("test"));

        let not_checkpoint = json!({"rewind": true});
        assert_eq!(CheckpointManager::is_checkpoint_signal(&not_checkpoint), None);
    }

    #[test]
    fn test_rewind_signal_detection() {
        let result = json!({"rewind": true, "label": "test"});
        assert_eq!(
            CheckpointManager::is_rewind_signal(&result),
            Some(Some("test"))
        );

        let no_label = json!({"rewind": true});
        assert_eq!(
            CheckpointManager::is_rewind_signal(&no_label),
            Some(None)
        );

        let not_rewind = json!({"checkpoint": true});
        assert_eq!(CheckpointManager::is_rewind_signal(&not_rewind), None);
    }
}
