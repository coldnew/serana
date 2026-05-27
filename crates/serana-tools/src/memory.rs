//! Persistent memory tools for Serana.
//!
//! Stores and recalls facts using SQLite with FTS5 for full-text search.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use serana_core::{Result, Tool};

use crate::ToolRegistry;

/// A stored fact row returned from queries.
struct FactRow {
    id: i64,
    fact_text: String,
    tags: String,
    created_at: String,
}

/// SQLite-backed persistent memory store.
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    /// Open or create the memory database at `dirs::data_dir()/serana/memory.db`.
    pub fn init() -> Result<Arc<Self>> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.create_tables()?;
        Ok(Arc::new(store))
    }

    fn db_path() -> Result<PathBuf> {
        let data_dir =
            dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))?;
        Ok(data_dir.join("serana").join("memory.db"))
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS facts (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                project     TEXT    NOT NULL,
                fact_text   TEXT    NOT NULL,
                tags        TEXT    NOT NULL DEFAULT '',
                created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
                fact_text,
                tags,
                content='facts',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON facts BEGIN
                INSERT INTO facts_fts(rowid, fact_text, tags)
                VALUES (new.id, new.fact_text, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, fact_text, tags)
                VALUES ('delete', old.id, old.fact_text, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, fact_text, tags)
                VALUES ('delete', old.id, old.fact_text, old.tags);
                INSERT INTO facts_fts(rowid, fact_text, tags)
                VALUES (new.id, new.fact_text, new.tags);
            END;
            ",
        )?;
        Ok(())
    }

    /// Insert a new fact for the given project.
    pub fn store_fact(&self, project: &str, fact_text: &str, tags: &str) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO facts (project, fact_text, tags) VALUES (?1, ?2, ?3)",
            params![project, fact_text, tags],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Full-text search across facts for the given project.
    pub fn search_facts(&self, project: &str, query: &str, limit: i64) -> Result<Vec<Value>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT f.id, f.fact_text, f.tags, f.created_at
             FROM facts_fts fts
             JOIN facts f ON f.id = fts.rowid
             WHERE facts_fts MATCH ?1 AND f.project = ?2
             ORDER BY f.id DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![query, project, limit], |row| {
            Ok(FactRow {
                id: row.get(0)?,
                fact_text: row.get(1)?,
                tags: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            let r = row?;
            results.push(json!({
                "id": r.id,
                "fact": r.fact_text,
                "tags": r.tags,
                "created_at": r.created_at,
            }));
        }
        Ok(results)
    }

    /// List recent facts for the given project, ordered by created_at DESC.
    pub fn list_facts(&self, project: &str, limit: i64) -> Result<Vec<Value>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, fact_text, tags, created_at
             FROM facts
             WHERE project = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project, limit], |row| {
            Ok(FactRow {
                id: row.get(0)?,
                fact_text: row.get(1)?,
                tags: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            let r = row?;
            results.push(json!({
                "id": r.id,
                "fact": r.fact_text,
                "tags": r.tags,
                "created_at": r.created_at,
            }));
        }
        Ok(results)
    }
}

/// Hash the current workspace path to derive a stable project identifier.
fn current_project() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cwd.to_string_lossy();
    // Simple hash: FNV-1a 64-bit
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

// ---------------------------------------------------------------------------
// RetainTool
// ---------------------------------------------------------------------------

/// Store a fact for future recall.
pub struct RetainTool {
    store: Arc<MemoryStore>,
}

impl RetainTool {
    pub fn new(store: Arc<MemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for RetainTool {
    fn name(&self) -> &'static str {
        "retain"
    }

    fn description(&self) -> &'static str {
        "Store a fact for future recall. Input: {\"fact\": \"...\", \"tags\": \"tag1,tag2\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "fact": {
                    "type": "string",
                    "description": "The fact to store"
                },
                "tags": {
                    "type": "string",
                    "description": "Comma-separated tags for categorization"
                }
            },
            "required": ["fact"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let fact = input
            .get("fact")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'fact' field"))?;
        let tags = input.get("tags").and_then(|v| v.as_str()).unwrap_or("");
        let project = current_project();
        let id = self.store.store_fact(&project, fact, tags)?;
        Ok(json!({ "stored": true, "id": id }))
    }
}

// ---------------------------------------------------------------------------
// RecallTool
// ---------------------------------------------------------------------------

/// Search stored facts using full-text search.
pub struct RecallTool {
    store: Arc<MemoryStore>,
}

impl RecallTool {
    pub fn new(store: Arc<MemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for RecallTool {
    fn name(&self) -> &'static str {
        "recall"
    }

    fn description(&self) -> &'static str {
        "Search stored facts. Input: {\"query\": \"...\", \"limit\": 10}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for full-text search across facts"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 10)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' field"))?;
        let limit = input.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let project = current_project();
        let results = self.store.search_facts(&project, query, limit)?;
        Ok(json!({ "facts": results, "count": results.len() }))
    }
}

// ---------------------------------------------------------------------------
// ReflectTool
// ---------------------------------------------------------------------------

/// List all stored facts.
pub struct ReflectTool {
    store: Arc<MemoryStore>,
}

impl ReflectTool {
    pub fn new(store: Arc<MemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for ReflectTool {
    fn name(&self) -> &'static str {
        "reflect"
    }

    fn description(&self) -> &'static str {
        "List all stored facts. Input: {\"limit\": 20}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of facts to return (default: 20)"
                }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let limit = input.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
        let project = current_project();
        let results = self.store.list_facts(&project, limit)?;
        Ok(json!({ "facts": results, "count": results.len() }))
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all memory tools. The `MemoryStore` is initialized once and shared.
pub fn register_memory_tools(registry: &mut ToolRegistry) -> Result<()> {
    let store = MemoryStore::init()?;
    registry.register(Box::new(RetainTool::new(Arc::clone(&store))));
    registry.register(Box::new(RecallTool::new(Arc::clone(&store))));
    registry.register(Box::new(ReflectTool::new(store)));
    Ok(())
}
