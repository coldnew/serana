use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, BufWriter};
use anyhow::Result;

pub struct History {
    entries: Vec<String>,
    index: usize,
    file_path: PathBuf,
    max_size: usize,
}

impl History {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let file_path = home.join(".mishell_history");
        let max_size = 10000;

        let entries = if file_path.exists() {
            Self::load_from_file(&file_path)?
        } else {
            Vec::new()
        };

        let index = entries.len();

        Ok(Self {
            entries,
            index,
            file_path,
            max_size,
        })
    }

    fn load_from_file(path: &PathBuf) -> Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.is_empty() {
                entries.push(line);
            }
        }

        Ok(entries)
    }

    pub fn add(&mut self, entry: &str) -> Result<()> {
        let entry = entry.trim().to_string();
        if entry.is_empty() {
            return Ok(());
        }

        // Don't add duplicates consecutively
        if self.entries.last() != Some(&entry) {
            self.entries.push(entry);

            // Trim if too large
            if self.entries.len() > self.max_size {
                self.entries.drain(0..self.entries.len() - self.max_size);
            }

            // Save to file
            self.save()?;
        }

        self.index = self.entries.len();
        Ok(())
    }

    fn save(&self) -> Result<()> {
        let file = File::create(&self.file_path)?;
        let mut writer = BufWriter::new(file);

        for entry in &self.entries {
            writeln!(writer, "{}", entry)?;
        }

        Ok(())
    }

    pub fn prev(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        if self.index > 0 {
            self.index -= 1;
        }
        Some(&self.entries[self.index])
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        if self.index < self.entries.len() - 1 {
            self.index += 1;
            Some(&self.entries[self.index])
        } else {
            self.index = self.entries.len();
            None
        }
    }

    #[allow(dead_code)]
    pub fn current_idx(&self) -> usize {
        self.index
    }

    pub fn search(&self, prefix: &str) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_history() -> (History, PathBuf) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mishell_test_history_{}_{}", std::process::id(), id));
        let _ = fs::remove_file(&path);
        let history = History {
            entries: Vec::new(),
            index: 0,
            file_path: path.clone(),
            max_size: 100,
        };
        (history, path)
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_new_empty() {
        let (history, path) = temp_history();
        assert!(history.entries().is_empty());
        assert_eq!(history.current_idx(), 0);
        cleanup(&path);
    }

    #[test]
    fn test_add_entry() {
        let (mut history, path) = temp_history();
        history.add("echo hello").unwrap();
        assert_eq!(history.entries().len(), 1);
        assert_eq!(history.entries()[0], "echo hello");
        cleanup(&path);
    }

    #[test]
    fn test_add_multiple() {
        let (mut history, path) = temp_history();
        history.add("echo a").unwrap();
        history.add("echo b").unwrap();
        history.add("echo c").unwrap();
        assert_eq!(history.entries().len(), 3);
        cleanup(&path);
    }

    #[test]
    fn test_add_empty_ignored() {
        let (mut history, path) = temp_history();
        history.add("").unwrap();
        history.add("   ").unwrap();
        assert!(history.entries().is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_no_consecutive_duplicates() {
        let (mut history, path) = temp_history();
        history.add("echo hello").unwrap();
        history.add("echo hello").unwrap();
        assert_eq!(history.entries().len(), 1);
        // Non-consecutive duplicate is OK
        history.add("echo world").unwrap();
        history.add("echo hello").unwrap();
        assert_eq!(history.entries().len(), 3);
        cleanup(&path);
    }

    #[test]
    fn test_prev_navigation() {
        let (mut history, path) = temp_history();
        history.add("first").unwrap();
        history.add("second").unwrap();
        history.add("third").unwrap();

        assert_eq!(history.prev(), Some("third"));
        assert_eq!(history.prev(), Some("second"));
        assert_eq!(history.prev(), Some("first"));
        assert_eq!(history.prev(), Some("first")); // stays at 0
        cleanup(&path);
    }

    #[test]
    fn test_next_navigation() {
        let (mut history, path) = temp_history();
        history.add("first").unwrap();
        history.add("second").unwrap();
        history.add("third").unwrap();

        // Go back
        history.prev();
        history.prev();
        history.prev();

        // Now go forward
        assert_eq!(history.next(), Some("second"));
        assert_eq!(history.next(), Some("third"));
        assert_eq!(history.next(), None); // past end
        cleanup(&path);
    }

    #[test]
    fn test_prev_on_empty() {
        let (mut history, path) = temp_history();
        assert_eq!(history.prev(), None);
        cleanup(&path);
    }

    #[test]
    fn test_next_on_empty() {
        let (mut history, path) = temp_history();
        assert_eq!(history.next(), None);
        cleanup(&path);
    }

    #[test]
    fn test_search() {
        let (mut history, path) = temp_history();
        history.add("echo hello").unwrap();
        history.add("echo world").unwrap();
        history.add("ls -la").unwrap();
        history.add("echo again").unwrap();

        let results: Vec<&str> = history.search("echo");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "echo hello");
        assert_eq!(results[1], "echo world");
        assert_eq!(results[2], "echo again");
        cleanup(&path);
    }

    #[test]
    fn test_search_no_match() {
        let (mut history, path) = temp_history();
        history.add("echo hello").unwrap();

        let results: Vec<&str> = history.search("grep");
        assert!(results.is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_search_prefix_only() {
        let (mut history, path) = temp_history();
        history.add("echo hello").unwrap();
        history.add("say echo").unwrap();

        let results: Vec<&str> = history.search("echo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "echo hello");
        cleanup(&path);
    }

    #[test]
    fn test_max_size_enforced() {
        let path = std::env::temp_dir().join(format!("mishell_test_history_max_{}", std::process::id()));
        let _ = fs::remove_file(&path);
        let mut history = History {
            entries: Vec::new(),
            index: 0,
            file_path: path.clone(),
            max_size: 3,
        };
        history.add("a").unwrap();
        history.add("b").unwrap();
        history.add("c").unwrap();
        history.add("d").unwrap();
        assert_eq!(history.entries().len(), 3);
        assert_eq!(history.entries()[0], "b");
        assert_eq!(history.entries()[2], "d");
        cleanup(&path);
    }

    #[test]
    fn test_save_and_load() {
        let (mut history, path) = temp_history();
        history.add("echo one").unwrap();
        history.add("echo two").unwrap();

        // Create a new history from the same file
        let loaded = History {
            entries: History::load_from_file(&path).unwrap(),
            index: 0,
            file_path: path.clone(),
            max_size: 100,
        };
        assert_eq!(loaded.entries().len(), 2);
        assert_eq!(loaded.entries()[0], "echo one");
        assert_eq!(loaded.entries()[1], "echo two");
        cleanup(&path);
    }

    #[test]
    fn test_current_idx_after_add() {
        let (mut history, path) = temp_history();
        history.add("first").unwrap();
        assert_eq!(history.current_idx(), 1);
        history.add("second").unwrap();
        assert_eq!(history.current_idx(), 2);
        cleanup(&path);
    }

    #[test]
    fn test_current_idx_after_prev() {
        let (mut history, path) = temp_history();
        history.add("first").unwrap();
        history.add("second").unwrap();
        history.prev();
        assert_eq!(history.current_idx(), 1);
        history.prev();
        assert_eq!(history.current_idx(), 0);
        cleanup(&path);
    }

    #[test]
    fn test_navigation_resets_on_add() {
        let (mut history, path) = temp_history();
        history.add("first").unwrap();
        history.add("second").unwrap();
        history.add("third").unwrap();
        history.prev();
        history.prev();
        assert_eq!(history.current_idx(), 1);

        // Adding should reset index to end
        history.add("fourth").unwrap();
        assert_eq!(history.current_idx(), 4);
        assert_eq!(history.prev(), Some("fourth"));
        cleanup(&path);
    }

    #[test]
    fn test_trim_whitespace_on_add() {
        let (mut history, path) = temp_history();
        history.add("  echo hello  ").unwrap();
        assert_eq!(history.entries()[0], "echo hello");
        cleanup(&path);
    }
}
