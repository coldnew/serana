use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write, BufWriter};
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
        if self.index > 0 {
            self.index -= 1;
            Some(&self.entries[self.index])
        } else {
            None
        }
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.index < self.entries.len() - 1 {
            self.index += 1;
            Some(&self.entries[self.index])
        } else {
            self.index = self.entries.len();
            None
        }
    }

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
