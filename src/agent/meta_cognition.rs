//! Meta-cognition for Serana self-improvement.
//!
//! Tracks modifications, learns from successes/failures, enables self-reflection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::Result;

/// Record of a self-modification made by Serana.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationRecord {
    /// Timestamp of modification
    pub timestamp: String,
    /// File that was modified
    pub file: String,
    /// Type of modification
    pub kind: ModificationKind,
    /// Description of what was changed
    pub description: String,
    /// Whether tests passed after modification
    pub tests_passed: bool,
    /// Git commit hash if committed
    pub commit: Option<String>,
    /// Lessons learned (filled after reflection)
    pub lessons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModificationKind {
    /// Added new functionality
    Feature,
    /// Fixed a bug
    BugFix,
    /// Improved performance
    Optimization,
    /// Refactored code
    Refactor,
    /// Added tests
    TestAddition,
    /// Updated dependencies
    Dependency,
    /// Configuration change
    Config,
}

/// Statistics about Serana's self-modifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModificationStats {
    pub total_modifications: u64,
    pub successful_modifications: u64,
    pub failed_modifications: u64,
    pub by_kind: HashMap<String, u64>,
    pub common_patterns: Vec<String>,
}

/// Meta-cognition system for tracking self-improvements.
pub struct MetaCognition {
    /// Path to modifications log
    log_path: PathBuf,
    /// In-memory cache of recent modifications
    recent: Vec<ModificationRecord>,
}

impl MetaCognition {
    /// Create a new meta-cognition system.
    pub fn new(workspace: PathBuf) -> Self {
        let log_path = workspace.join(".serana").join("modifications.jsonl");
        Self {
            log_path,
            recent: Vec::new(),
        }
    }

    /// Record a modification made by Serana.
    pub async fn record(&mut self, record: ModificationRecord) -> Result<()> {
        // Append to log file
        let line = serde_json::to_string(&record)? + "\n";
        
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        
        // Update cache
        self.recent.push(record);
        if self.recent.len() > 100 {
            self.recent.remove(0);
        }
        
        Ok(())
    }

    /// Get statistics about modifications.
    pub async fn stats(&self) -> Result<ModificationStats> {
        let records = self.load_all().await?;
        
        let mut stats = ModificationStats::default();
        
        for record in &records {
            stats.total_modifications += 1;
            
            if record.tests_passed {
                stats.successful_modifications += 1;
            } else {
                stats.failed_modifications += 1;
            }
            
            let kind_key = format!("{:?}", record.kind);
            *stats.by_kind.entry(kind_key).or_insert(0) += 1;
        }
        
        // Extract common patterns from lessons
        let mut lesson_counts: HashMap<String, u64> = HashMap::new();
        for record in &records {
            for lesson in &record.lessons {
                *lesson_counts.entry(lesson.clone()).or_insert(0) += 1;
            }
        }
        
        let mut lessons: Vec<_> = lesson_counts.into_iter().collect();
        lessons.sort_by(|a, b| b.1.cmp(&a.1));
        stats.common_patterns = lessons.into_iter().take(10).map(|(k, _)| k).collect();
        
        Ok(stats)
    }

    /// Get recent modifications.
    pub fn recent(&self, limit: usize) -> &[ModificationRecord] {
        let start = self.recent.len().saturating_sub(limit);
        &self.recent[start..]
    }

    /// Reflect on a modification and add lessons learned.
    pub async fn reflect(&mut self, file: &str, lessons: Vec<String>) -> Result<()> {
        // Find the most recent modification to this file
        if let Some(record) = self.recent.iter_mut().rev().find(|r| r.file == file) {
            record.lessons = lessons.clone();
            
            // Rewrite the log with updated record
            self.rewrite_log().await?;
        }
        
        Ok(())
    }

    /// Load all modification records from disk.
    async fn load_all(&self) -> Result<Vec<ModificationRecord>> {
        let mut records = Vec::new();
        
        if !self.log_path.exists() {
            return Ok(records);
        }
        
        let content = fs::read_to_string(&self.log_path).await?;
        for line in content.lines() {
            if let Ok(record) = serde_json::from_str::<ModificationRecord>(line) {
                records.push(record);
            }
        }
        
        Ok(records)
    }

    /// Rewrite the log file (used when updating records).
    async fn rewrite_log(&self) -> Result<()> {
        let mut content = String::new();
        for record in &self.recent {
            content += &serde_json::to_string(record)?;
            content += "\n";
        }
        
        fs::write(&self.log_path, content).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn records_modification() {
        let dir = tempdir().unwrap();
        let mut meta = MetaCognition::new(dir.path().to_path_buf());
        
        let record = ModificationRecord {
            timestamp: "2026-05-21T00:00:00Z".to_string(),
            file: "src/agent/coding.rs".to_string(),
            kind: ModificationKind::Feature,
            description: "Added self-reflection".to_string(),
            tests_passed: true,
            commit: Some("abc123".to_string()),
            lessons: vec![],
        };
        
        meta.record(record).await.unwrap();
        assert!(!meta.recent.is_empty());
    }

    #[tokio::test]
    async fn computes_stats() {
        let dir = tempdir().unwrap();
        let mut meta = MetaCognition::new(dir.path().to_path_buf());
        
        meta.record(ModificationRecord {
            timestamp: "2026-05-21T00:00:00Z".into(),
            file: "src/lib.rs".into(),
            kind: ModificationKind::Feature,
            description: "Test 1".into(),
            tests_passed: true,
            commit: None,
            lessons: vec!["Test early".into()],
        }).await.unwrap();
        
        meta.record(ModificationRecord {
            timestamp: "2026-05-21T00:00:01Z".into(),
            file: "src/lib.rs".into(),
            kind: ModificationKind::BugFix,
            description: "Test 2".into(),
            tests_passed: false,
            commit: None,
            lessons: vec![],
        }).await.unwrap();
        
        let stats = meta.stats().await.unwrap();
        assert_eq!(stats.total_modifications, 2);
        assert_eq!(stats.successful_modifications, 1);
        assert_eq!(stats.failed_modifications, 1);
    }
}
