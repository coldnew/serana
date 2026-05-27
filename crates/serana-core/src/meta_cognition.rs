use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

use crate::Result;

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
    pub total_modifications: u64,
    pub successful_modifications: u64,
    pub failed_modifications: u64,
    pub by_kind: HashMap<String, u64>,
    pub common_patterns: Vec<String>,
    pub total_tool_calls: u64,
    pub total_lessons: u64,
    pub top_failing_tools: Vec<(String, u64)>,
    pub top_modified_files: Vec<(String, u64)>,
}

/// Record for meta-cognition tracking (compressed view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaRecord {
    pub timestamp: String,
    pub action: String,
    pub details: String,
    pub outcome: String,
}

/// Meta-cognition system for tracking decisions and learning from experience.
pub struct MetaCognition {
    history: Mutex<Vec<ModificationRecord>>,
    lessons: Mutex<Vec<(String, String)>>, // (timestamp, lesson)
    learning_enabled: Mutex<bool>,
    lesson_count: Mutex<usize>,
    persistence_path: Option<PathBuf>,
}

impl MetaCognition {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            lessons: Mutex::new(Vec::new()),
            learning_enabled: Mutex::new(true),
            lesson_count: Mutex::new(0),
            persistence_path: None,
        }
    }

    /// Enable learning mode.
    pub fn enable_learning(&self) {
        let mut enabled = self.learning_enabled.blocking_lock();
        *enabled = true;
    }

    /// Disable learning mode.
    pub fn disable_learning(&self) {
        let mut enabled = self.learning_enabled.blocking_lock();
        *enabled = false;
    }

    /// Record a modification.
    pub async fn record(&self, record: ModificationRecord) -> Result<()> {
        let mut history = self.history.lock().await;
        history.push(record);
        let excess = history.len().saturating_sub(1000);
        if excess > 0 {
            history.drain(0..excess);
        }
        Ok(())
    }

    /// Add a lesson learned.
    pub async fn add_lesson(&self, timestamp: &str, lesson: String) -> Result<()> {
        let enabled = self.learning_enabled.lock().await;
        if !*enabled {
            return Ok(());
        }
        drop(enabled);
        let mut lessons = self.lessons.lock().await;
        lessons.push((timestamp.to_string(), lesson));
        let mut count = self.lesson_count.lock().await;
        *count = lessons.len();
        Ok(())
    }

    /// Get recent failures for a specific tool.
    pub async fn get_recent_failures(
        &self,
        tool_name: &str,
        count: usize,
    ) -> Vec<ModificationRecord> {
        let mut failures = Vec::new();
        let history = self.history.lock().await;
        for record in history.iter().rev() {
            if record.description.contains(tool_name) && !record.tests_passed {
                failures.push(record.clone());
                if failures.len() >= count {
                    break;
                }
            }
        }
        failures
    }

    /// Get recent lessons.
    pub async fn get_recent_lessons(&self, count: usize) -> Vec<String> {
        let mut recent = Vec::new();
        let lessons = self.lessons.lock().await;
        for (_, lesson) in lessons.iter().rev().take(count) {
            recent.push(lesson.clone());
        }
        recent
    }

    /// Get lessons for a specific file.
    pub async fn get_lessons_for_file(&self, file_path: &str) -> Vec<String> {
        let mut file_lessons = Vec::new();
        let history = self.history.lock().await;
        for record in history.iter().rev() {
            if record.file == file_path {
                file_lessons.extend(record.lessons.clone());
            }
        }
        file_lessons
    }

    /// Get modification stats.
    pub async fn get_stats(&self) -> ModificationStats {
        let mut stats = ModificationStats::default();

        let history = self.history.lock().await;
        stats.total_modifications = history.len() as u64;
        stats.successful_modifications = history.iter().filter(|r| r.tests_passed).count() as u64;
        stats.failed_modifications = history.iter().filter(|r| !r.tests_passed).count() as u64;

        let mut tool_failures: HashMap<String, u64> = HashMap::new();
        let mut file_mods: HashMap<String, u64> = HashMap::new();
        let mut tool_calls = 0u64;
        let mut by_kind: HashMap<String, u64> = HashMap::new();

        for record in history.iter() {
            if record.kind == ModificationKind::ToolCall {
                tool_calls += 1;
                if !record.tests_passed {
                    *tool_failures.entry(record.file.clone()).or_default() += 1;
                }
            }
            *file_mods.entry(record.file.clone()).or_default() += 1;
            *by_kind.entry(format!("{:?}", record.kind)).or_default() += 1;
        }

        stats.by_kind = by_kind;

        stats.total_tool_calls = tool_calls;
        let mut failures: Vec<_> = tool_failures.into_iter().collect();
        failures.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_failing_tools = failures.into_iter().take(5).collect();

        let mut files: Vec<_> = file_mods.into_iter().collect();
        files.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_modified_files = files.into_iter().take(5).collect();

        drop(history);
        let count = self.lesson_count.lock().await;
        stats.total_lessons = *count as u64;

        stats
    }

    pub async fn get_decision_history(&self, file: &str) -> Vec<ModificationRecord> {
        let history = self.history.lock().await;
        let records = history
            .iter()
            .rev()
            .take(50)
            .filter(|r| r.file == file)
            .cloned()
            .collect();
        records
    }

    pub async fn reflect(&self, _file: &str, lessons: Vec<String>) -> Result<()> {
        for lesson in lessons {
            self.add_lesson("reflect", lesson).await?;
        }
        Ok(())
    }

    /// Persist meta-cognition data to disk.
    pub async fn persist(&self) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            let history = self.history.lock().await;
            let data = serde_json::to_string_pretty(&*history)?;
            std::fs::write(path, data)?;
        }
        Ok(())
    }

    /// Set the persistence path.
    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persistence_path = Some(path);
        self
    }

    /// Get the history for analysis.
    pub async fn get_history(&self) -> Vec<ModificationRecord> {
        let h = self.history.lock().await;
        h.clone()
    }
}

impl Default for MetaCognition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_modifications() {
        let mc = MetaCognition::new();
        let record = ModificationRecord {
            timestamp: "1234567890".to_string(),
            file: "test.rs".to_string(),
            kind: ModificationKind::Feature,
            description: "Added test feature".to_string(),
            tests_passed: true,
            commit: None,
            lessons: vec![],
        };

        mc.record(record).await.unwrap();
        let history = mc.get_history().await;
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn tracks_lessons() {
        let mc = MetaCognition::new();
        mc.add_lesson("1234567890", "Always check for nulls".to_string())
            .await
            .unwrap();

        let lessons = mc.get_recent_lessons(10).await;
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0], "Always check for nulls");
    }

    #[tokio::test]
    async fn reports_stats() {
        let mc = MetaCognition::new();
        let record = ModificationRecord {
            timestamp: "1234567890".to_string(),
            file: "tool:read_file".to_string(),
            kind: ModificationKind::ToolCall,
            description: "Read file failed".to_string(),
            tests_passed: false,
            commit: None,
            lessons: vec![],
        };
        mc.record(record).await.unwrap();

        let stats = mc.get_stats().await;
        assert_eq!(stats.total_modifications, 1);
        assert_eq!(stats.failed_modifications, 1);
    }

    #[tokio::test]
    async fn limits_history_size() {
        let mc = MetaCognition::new();
        let record_template = ModificationRecord {
            timestamp: "test".to_string(),
            file: "test.rs".to_string(),
            kind: ModificationKind::Feature,
            description: "test".to_string(),
            tests_passed: true,
            commit: None,
            lessons: vec![],
        };

        for _ in 0..1500 {
            mc.record(record_template.clone()).await.unwrap();
        }

        let history = mc.get_history().await;
        assert_eq!(history.len(), 1000);
    }
}
