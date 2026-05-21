//! Session persistence with SQLite storage.
//!
//! Stores conversation history, tool calls, compression lineage, and FTS5 search.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type SessionId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub tool_call_count: u32,
    pub compressed_from: Option<SessionId>,
    pub compression_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub meta: SessionMeta,
    pub messages: Vec<StoredMessage>,
    pub tool_calls: Vec<StoredToolCall>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: SessionId,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    db_path: PathBuf,
}

impl SessionStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn default_location() -> Self {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("serana");
        let _ = std::fs::create_dir_all(&dir);
        Self::new(dir.join("sessions.db"))
    }

    pub fn init(&self) -> crate::Result<()> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                tool_call_count INTEGER NOT NULL DEFAULT 0,
                compressed_from TEXT REFERENCES sessions(id),
                compression_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tool_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                arguments TEXT NOT NULL,
                result TEXT,
                timestamp TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                content='messages',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;
            "#,
        )?;
        Ok(())
    }

    pub fn create_session(&self) -> crate::Result<Session> {
        let now = Utc::now();
        let id = format!("sess_{}", uuid::Uuid::new_v4());
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, now.to_rfc3339(), now.to_rfc3339()],
        )?;

        Ok(Session {
            meta: SessionMeta {
                id,
                created_at: now,
                updated_at: now,
                message_count: 0,
                tool_call_count: 0,
                compressed_from: None,
                compression_count: 0,
            },
            messages: Vec::new(),
            tool_calls: Vec::new(),
        })
    }

    pub fn create_compressed_session(&self, compressed_from: &str) -> crate::Result<Session> {
        let mut session = self.create_session()?;
        let count = self
            .load_session(compressed_from)?
            .map(|s| s.meta.compression_count.saturating_add(1))
            .unwrap_or(1);

        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE sessions SET compressed_from = ?1, compression_count = ?2 WHERE id = ?3",
            rusqlite::params![compressed_from, count, session.meta.id],
        )?;
        session.meta.compressed_from = Some(compressed_from.to_string());
        session.meta.compression_count = count;
        Ok(session)
    }

    pub fn load_session(&self, id: &str) -> crate::Result<Option<Session>> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let meta = match conn.query_row(
            "SELECT id, created_at, updated_at, message_count, tool_call_count, compressed_from, compression_count FROM sessions WHERE id = ?1",
            [id],
            read_meta,
        ) {
            Ok(meta) => meta,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(err) => return Err(err.into()),
        };

        let mut messages_stmt = conn.prepare(
            "SELECT role, content, timestamp FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let messages = messages_stmt
            .query_map([id], |row| {
                Ok(StoredMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                    timestamp: parse_time(row.get(2)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut tools_stmt = conn.prepare(
            "SELECT name, arguments, result, timestamp FROM tool_calls WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let tool_calls = tools_stmt
            .query_map([id], |row| {
                let arguments: String = row.get(1)?;
                let result: Option<String> = row.get(2)?;
                Ok(StoredToolCall {
                    name: row.get(0)?,
                    arguments: serde_json::from_str(&arguments).unwrap_or(serde_json::Value::Null),
                    result: result.and_then(|value| serde_json::from_str(&value).ok()),
                    timestamp: parse_time(row.get(3)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(Session { meta, messages, tool_calls }))
    }

    pub fn save_message(&self, session_id: &str, role: &str, content: &str) -> crate::Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![session_id, role, content, now],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?1, message_count = message_count + 1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    pub fn save_tool_call(
        &self,
        session_id: &str,
        name: &str,
        arguments: &serde_json::Value,
        result: Option<&serde_json::Value>,
    ) -> crate::Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO tool_calls (session_id, name, arguments, result, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session_id,
                name,
                serde_json::to_string(arguments)?,
                result.map(serde_json::to_string).transpose()?,
                now,
            ],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?1, tool_call_count = tool_call_count + 1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    pub fn search_messages(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT m.session_id, m.role, m.content, m.timestamp FROM messages_fts f JOIN messages m ON m.id = f.rowid WHERE messages_fts MATCH ?1 ORDER BY m.id DESC LIMIT ?2",
        )?;
        let results = stmt
            .query_map(rusqlite::params![query, limit as i64], |row| {
                Ok(SearchResult {
                    session_id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    timestamp: parse_time(row.get(3)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    pub fn list_recent_sessions(&self, limit: usize) -> crate::Result<Vec<SessionMeta>> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, message_count, tool_call_count, compressed_from, compression_count FROM sessions ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let sessions = stmt
            .query_map([limit as i64], read_meta)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }
}

fn read_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionMeta> {
    Ok(SessionMeta {
        id: row.get(0)?,
        created_at: parse_time(row.get(1)?),
        updated_at: parse_time(row.get(2)?),
        message_count: row.get(3)?,
        tool_call_count: row.get(4)?,
        compressed_from: row.get(5)?,
        compression_count: row.get(6)?,
    })
}

fn parse_time(value: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn stores_and_loads_sessions() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions.db"));
        store.init().unwrap();

        let session = store.create_session().unwrap();
        store.save_message(&session.meta.id, "user", "How do I implement OAuth?").unwrap();
        store.save_tool_call(
            &session.meta.id,
            "read_file",
            &serde_json::json!({"path":"Cargo.toml"}),
            Some(&serde_json::json!({"ok":true})),
        ).unwrap();

        let loaded = store.load_session(&session.meta.id).unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.tool_calls.len(), 1);
        assert_eq!(loaded.meta.message_count, 1);
        assert_eq!(loaded.meta.tool_call_count, 1);
    }

    #[test]
    fn searches_messages_with_fts5() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions.db"));
        store.init().unwrap();

        let session = store.create_session().unwrap();
        store.save_message(&session.meta.id, "user", "OAuth implementation details").unwrap();

        let results = store.search_messages("OAuth", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session.meta.id);
    }

    #[test]
    fn tracks_compression_lineage() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path().join("sessions.db"));
        store.init().unwrap();

        let original = store.create_session().unwrap();
        let compressed = store.create_compressed_session(&original.meta.id).unwrap();

        assert_eq!(compressed.meta.compressed_from.as_deref(), Some(original.meta.id.as_str()));
        assert_eq!(compressed.meta.compression_count, 1);
    }
}
