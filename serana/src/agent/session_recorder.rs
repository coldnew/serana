use std::sync::Arc;

use crate::core::{Result, ToolCall};
use crate::llm::AuxiliaryClient;

use super::SessionStore;

#[derive(Clone, Default)]
pub struct SessionRecorder {
    store: Option<SessionStore>,
    session_id: Option<String>,
}

impl SessionRecorder {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn new(store: SessionStore, session_id: String) -> Self {
        Self {
            store: Some(store),
            session_id: Some(session_id),
        }
    }

    pub fn save_message(&self, role: &str, content: &str) -> Result<()> {
        if let (Some(store), Some(session_id)) = (&self.store, &self.session_id) {
            store.save_message(session_id, role, content)?;
        }
        Ok(())
    }

    pub fn save_tool_call(&self, tool_call: &ToolCall) -> Result<()> {
        if let (Some(store), Some(session_id)) = (&self.store, &self.session_id) {
            store.save_tool_call(
                session_id,
                &tool_call.name,
                &tool_call.arguments,
                tool_call.result.as_ref(),
            )?;
        }
        Ok(())
    }

    pub fn generate_title_async(&self, auxiliary: Option<Arc<AuxiliaryClient>>, instruction: &str) {
        let (Some(store), Some(session_id), Some(auxiliary)) =
            (&self.store, &self.session_id, auxiliary)
        else {
            return;
        };

        let store = store.clone();
        let session_id = session_id.clone();
        let first_msg = instruction.chars().take(200).collect::<String>();
        tokio::spawn(async move {
            if let Ok(title) = auxiliary.generate_title(&first_msg).await {
                let _ = store.update_session_title(&session_id, &title);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::test_support::tempdir;

    #[test]
    fn disabled_recorder_is_noop() {
        let recorder = SessionRecorder::disabled();
        recorder.save_message("user", "hello").unwrap();
    }

    #[test]
    fn records_messages_and_tool_calls_when_enabled() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions.db"));
        store.init().unwrap();
        let session = store.create_session().unwrap();
        let recorder = SessionRecorder::new(store.clone(), session.meta.id.clone());

        recorder.save_message("user", "hello").unwrap();
        recorder
            .save_tool_call(&ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path":"Cargo.toml"}),
                result: Some(serde_json::json!({"ok":true})),
            })
            .unwrap();

        let loaded = store.load_session(&session.meta.id).unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.tool_calls.len(), 1);
    }
}
