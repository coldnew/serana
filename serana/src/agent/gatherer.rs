use std::path::{Path, PathBuf};

use crate::core::Result;
use ignore::WalkBuilder;

/// Gathers relevant workspace context for the agent's system prompt.
///
/// Scans the workspace directory (respecting .gitignore) and produces
/// a file tree plus identifies files relevant to the current instruction.
#[derive(Debug, Clone)]
pub struct ContextGatherer {
    max_tree_entries: usize,
    max_depth: usize,
}

impl ContextGatherer {
    pub fn new() -> Self {
        Self {
            max_tree_entries: 200,
            max_depth: 4,
        }
    }

    /// Gather file paths relevant to the given instruction.
    /// Uses keyword extraction from the instruction to match file names and paths.
    pub async fn gather_relevant_files(
        &self,
        workspace: &PathBuf,
        instruction: &str,
    ) -> Result<Vec<PathBuf>> {
        let keywords = extract_keywords(instruction);
        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        let mut matches = Vec::new();
        let walker = WalkBuilder::new(workspace)
            .max_depth(Some(self.max_depth))
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |f| f.is_file()) {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy().to_lowercase();

            for kw in &keywords {
                if path_str.contains(kw) {
                    matches.push(path.to_path_buf());
                    break;
                }
            }

            if matches.len() >= 50 {
                break;
            }
        }

        Ok(matches)
    }

    /// Build a text representation of the workspace file tree.
    /// Used to include a project overview in the system prompt.
    pub fn build_workspace_tree(&self, workspace: &Path) -> String {
        let mut entries = Vec::new();
        let walker = WalkBuilder::new(workspace)
            .max_depth(Some(self.max_depth))
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if entries.len() >= self.max_tree_entries {
                entries.push("... (truncated)".to_string());
                break;
            }

            let path = entry.path();
            let relative = path
                .strip_prefix(workspace)
                .unwrap_or(path)
                .to_string_lossy();

            if relative.is_empty() {
                continue;
            }

            let depth = path
                .strip_prefix(workspace)
                .unwrap_or(path)
                .components()
                .count()
                - 1;

            let prefix = "  ".repeat(depth);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if entry.file_type().map_or(false, |f| f.is_dir()) {
                entries.push(format!("{}{}/", prefix, name));
            } else {
                entries.push(format!("{}{}", prefix, name));
            }
        }

        entries.join("\n")
    }

    /// Gather project metadata (Cargo.toml, package.json, etc.) as context strings.
    pub fn gather_project_metadata(&self, workspace: &Path) -> Vec<String> {
        let mut metadata = Vec::new();

        // Check for Rust project
        let cargo_toml = workspace.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                // Extract just the [package] section for brevity
                if let Some(pkg_start) = content.find("[package]") {
                    let rest = &content[pkg_start..];
                    let end = rest.find("\n[").unwrap_or(rest.len());
                    metadata.push(format!("Cargo.toml:\n{}", &rest[..end]));
                }
            }
        }

        // Check for Node project
        let package_json = workspace.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                // Just include first 500 chars
                let truncated = if content.len() > 500 {
                    &content[..500]
                } else {
                    &content
                };
                metadata.push(format!("package.json:\n{}", truncated));
            }
        }

        // Check for Go project
        let go_mod = workspace.join("go.mod");
        if go_mod.exists() {
            if let Ok(content) = std::fs::read_to_string(&go_mod) {
                metadata.push(format!("go.mod:\n{}", content));
            }
        }

        // Check for Python project
        let pyproject = workspace.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                let truncated = if content.len() > 500 {
                    &content[..500]
                } else {
                    &content
                };
                metadata.push(format!("pyproject.toml:\n{}", truncated));
            }
        }

        metadata
    }
}

impl Default for ContextGatherer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract meaningful keywords from an instruction for file matching.
fn extract_keywords(instruction: &str) -> Vec<String> {
    let stop_words: &[&str] = &[
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "shall",
        "can",
        "need",
        "must",
        "ought",
        "i",
        "you",
        "he",
        "she",
        "it",
        "we",
        "they",
        "me",
        "him",
        "her",
        "us",
        "them",
        "my",
        "your",
        "his",
        "its",
        "our",
        "their",
        "mine",
        "yours",
        "hers",
        "ours",
        "theirs",
        "this",
        "that",
        "these",
        "those",
        "and",
        "or",
        "but",
        "if",
        "then",
        "else",
        "when",
        "where",
        "how",
        "what",
        "which",
        "who",
        "whom",
        "why",
        "not",
        "no",
        "nor",
        "so",
        "for",
        "of",
        "to",
        "from",
        "by",
        "with",
        "in",
        "on",
        "at",
        "up",
        "out",
        "off",
        "over",
        "under",
        "about",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "each",
        "few",
        "more",
        "most",
        "other",
        "some",
        "such",
        "than",
        "too",
        "very",
        "just",
        "also",
        "now",
        "here",
        "there",
        "all",
        "any",
        "both",
        "every",
        "only",
        "own",
        "same",
        "let",
        "make",
        "like",
        "get",
        "go",
        "see",
        "know",
        "take",
        "come",
        "think",
        "look",
        "want",
        "give",
        "use",
        "find",
        "tell",
        "ask",
        "work",
        "seem",
        "feel",
        "try",
        "leave",
        "call",
        "please",
        "help",
        "want",
        "need",
        "file",
        "code",
        "function",
        "implement",
        "fix",
        "change",
        "update",
        "add",
        "remove",
        "create",
        "write",
        "read",
        "run",
        "test",
    ];

    instruction
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .to_lowercase()
        })
        .filter(|w| w.len() >= 3 && !stop_words.contains(&w.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_keywords() {
        let kw = extract_keywords("fix the bug in parser.rs where tokens are not matched");
        assert!(kw.contains(&"parser.rs".to_string()));
        assert!(kw.contains(&"tokens".to_string()));
        assert!(kw.contains(&"matched".to_string()));
        assert!(!kw.contains(&"the".to_string()));
        assert!(!kw.contains(&"in".to_string()));
    }

    #[test]
    fn test_build_workspace_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let gatherer = ContextGatherer::new();
        let tree = gatherer.build_workspace_tree(root);

        assert!(tree.contains("src/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("Cargo.toml"));
    }

    #[test]
    fn test_gather_project_metadata_rust() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .unwrap();

        let gatherer = ContextGatherer::new();
        let meta = gatherer.gather_project_metadata(dir.path());

        assert_eq!(meta.len(), 1);
        assert!(meta[0].contains("test"));
    }

    #[tokio::test]
    async fn test_gather_relevant_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/parser.rs"), "// parser").unwrap();
        fs::write(root.join("src/lexer.rs"), "// lexer").unwrap();
        fs::write(root.join("src/main.rs"), "// main").unwrap();

        let gatherer = ContextGatherer::new();
        let files = gatherer
            .gather_relevant_files(&root.to_path_buf(), "fix the parser bug")
            .await
            .unwrap();

        assert!(files.iter().any(|p| p.to_string_lossy().contains("parser")));
    }
}
