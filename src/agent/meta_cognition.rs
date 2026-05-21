//! Meta-cognition for agent self-reflection and decision tracking.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Type of modification or decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModificationKind {
    ToolCall,
    ContextCompression,
    Iteration,
    Cancellation,
    Error,
    Decision,
    Observation,
    Feature,
    BugFix,
    Optimization,
    Refactor,
    TestAddition,
    Dependency,
    Config,
}

/// Full modification record with lessons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationRecord {
    pub timestamp: String,
    pub file: String,
    pub kind: ModificationKind,
    pub description: String,
    pub tests_passed: bool,
    pub commit: Option<String>,
    pub lessons: Vec<String>,
}

/// Stats for self-evolution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModificationStats {
    pub total_modifications: usize,
    pub successful_modifications: usize,
    pub failed_modifications: usize,
    pub by_kind: HashMap<String, usize>,
    pub common_patterns: Vec<String>,
}

/// Simple in-memory record for transient events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaRecord {
    pub id: u64,
    pub kind: ModificationKind,
    pub description: String,
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

/// Meta-cognition state with persistence support.
#[derive(Debug)]
pub struct MetaCognition {
    records: Mutex<VecDeque<MetaRecord>>,
    next_id: Mutex<u64>,
    max_records: usize,
    storage_path: PathBuf,
    persisted_records: Mutex<Vec<ModificationRecord>>,
}

impl MetaCognition {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let storage_dir = PathBuf::from(home).join(".serana");
        Self::with_storage(storage_dir)
    }

    pub fn with_storage<P: AsRef<Path>>(storage_dir: P) -> Self {
        let storage_path = storage_dir.as_ref().join("modifications.json");
        if let Some(parent) = storage_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let persisted = Self::load_persisted(&storage_path).unwrap_or_default();
        Self {
            records: Mutex::new(VecDeque::with_capacity(1000)),
            next_id: Mutex::new(0),
            max_records: 1000,
            storage_path,
            persisted_records: Mutex::new(persisted),
        }
    }

    fn load_persisted(path: &Path) -> Option<Vec<ModificationRecord>> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn save_persisted(&self, records: &[ModificationRecord]) -> crate::Result<()> {
        let json = serde_json::to_string_pretty(records)?;
        tokio::fs::write(&self.storage_path, json).await?;
        Ok(())
    }

    pub async fn add_record(
        &self,
        kind: ModificationKind,
        description: impl Into<String>,
        metadata: serde_json::Value,
    ) -> u64 {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;
        drop(next_id);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let record = MetaRecord {
            id,
            kind,
            description: description.into(),
            timestamp,
            metadata,
        };
        let mut records = self.records.lock().await;
        if records.len() >= self.max_records {
            records.pop_front();
        }
        records.push_back(record);
        id
    }

    pub async fn record(&self, record: ModificationRecord) -> crate::Result<()> {
        let mut records = self.persisted_records.lock().await;
        records.push(record);
        self.save_persisted(&records).await
    }

    pub async fn stats(&self) -> crate::Result<ModificationStats> {
        let records = self.persisted_records.lock().await;
        let total = records.len();
        let successful = records.iter().filter(|r| r.tests_passed).count();
        let mut by_kind: HashMap<String, usize> = HashMap::new();
        for r in records.iter() {
            *by_kind.entry(format!("{:?}", r.kind)).or_insert(0) += 1;
        }
        Ok(ModificationStats {
            total_modifications: total,
            successful_modifications: successful,
            failed_modifications: total - successful,
            by_kind,
            common_patterns: vec![],
        })
    }

    pub async fn reflect(&self, file: &str, lessons: Vec<String>) -> crate::Result<()> {
        let mut records = self.persisted_records.lock().await;
        if let Some(record) = records.iter_mut().rev().find(|r| r.file == file) {
            record.lessons.extend(lessons);
            self.save_persisted(&records).await?;
        }
        Ok(())
    }

    pub async fn get_last(&self, n: usize) -> Vec<MetaRecord> {
        let records = self.records.lock().await;
        let start = records.len().saturating_sub(n);
        records.iter().skip(start).cloned().collect()
    }

    pub async fn query_by_context(&self, query: &str) -> Vec<ModificationRecord> {
        let records = self.persisted_records.lock().await;
        records
            .iter()
            .filter(|r| {
                r.file.contains(query)
                    || r.description.contains(query)
                    || format!("{:?}", r.kind).contains(query)
            })
            .cloned()
            .collect()
    }

    pub async fn get_recent_failures(&self, tool_name: &str, limit: usize) -> Vec<ModificationRecord> {
        let records = self.persisted_records.lock().await;
        records
            .iter()
            .filter(|r| {
                r.file == format!("tool:{}", tool_name) && !r.tests_passed
            })
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn add_lesson(&self, timestamp: &str, lesson: String) -> crate::Result<()> {
        let mut records = self.persisted_records.lock().await;
        if let Some(record) = records.iter_mut().find(|r| r.timestamp == timestamp) {
            record.lessons.push(lesson);
            self.save_persisted(&records).await?;
            Ok(())
        } else {
            anyhow::bail!("Record not found for timestamp: {}", timestamp)
        }
    }
}