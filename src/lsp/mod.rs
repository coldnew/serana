//! LSP (Language Server Protocol) client implementation
//! 
//! Provides built-in LSP support for code intelligence features like
//! go-to-definition, find-references, hover, rename, and diagnostics.

pub mod client;
pub mod transport;
pub mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::Result;

/// Language identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}

impl LanguageId {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" => Some(Self::JavaScript),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn lsp_language_id(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
        }
    }

    pub fn server_command(&self) -> &'static str {
        match self {
            Self::Rust => "rust-analyzer",
            Self::TypeScript | Self::JavaScript => "typescript-language-server",
            Self::Python => "pylsp",
            Self::Go => "gopls",
        }
    }
}

/// LSP manager that handles multiple language servers
pub struct LspManager {
    workspace_root: PathBuf,
    servers: HashMap<LanguageId, client::LspClient>,
}

impl LspManager {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            servers: HashMap::new(),
        }
    }

    /// Detect which language servers should be started based on workspace
    pub fn detect_language_servers(&self) -> Vec<LanguageId> {
        let mut languages = Vec::new();

        // Check for Cargo.toml -> Rust
        if self.workspace_root.join("Cargo.toml").exists() {
            languages.push(LanguageId::Rust);
        }

        // Check for package.json -> TypeScript/JavaScript
        if self.workspace_root.join("package.json").exists() {
            languages.push(LanguageId::TypeScript);
        }

        // Check for go.mod -> Go
        if self.workspace_root.join("go.mod").exists() {
            languages.push(LanguageId::Go);
        }

        // Check for requirements.txt or pyproject.toml -> Python
        if self.workspace_root.join("requirements.txt").exists()
            || self.workspace_root.join("pyproject.toml").exists()
        {
            languages.push(LanguageId::Python);
        }

        languages
    }

    pub async fn definition(&mut self, path: &std::path::Path, position: types::Position) -> Result<Vec<types::Location>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .definition(path, position)
            .await
    }

    pub async fn references(&mut self, path: &std::path::Path, position: types::Position) -> Result<Vec<types::Location>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .references(path, position)
            .await
    }

    pub async fn hover(&mut self, path: &std::path::Path, position: types::Position) -> Result<Option<String>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .hover(path, position)
            .await
    }

    async fn ensure_server(&mut self, lang: LanguageId) -> Result<()> {
        if !self.servers.contains_key(&lang) {
            self.start_server(lang).await?;
        }
        Ok(())
    }


    /// Start a language server for the given language
    pub async fn start_server(&mut self, lang: LanguageId) -> Result<()> {
        let command = lang.server_command();
        let client = client::LspClient::spawn(command, &self.workspace_root).await?;
        self.servers.insert(lang, client);
        Ok(())
    }

    /// Get LSP client for a language
    pub fn get_client(&self, lang: LanguageId) -> Option<&client::LspClient> {
        self.servers.get(&lang)
    }

    /// Get mutable LSP client for a language
    pub fn get_client_mut(&mut self, lang: LanguageId) -> Option<&mut client::LspClient> {
        self.servers.get_mut(&lang)
    }

    /// Shutdown all language servers
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (_lang, client) in self.servers.drain() {
            client.shutdown().await?;
        }
        Ok(())
    }
}

fn language_for_path(path: &std::path::Path) -> Result<LanguageId> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| anyhow::anyhow!("missing file extension for {}", path.display()))?;
    LanguageId::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("unsupported source language: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(LanguageId::from_extension("rs"), Some(LanguageId::Rust));
        assert_eq!(LanguageId::from_extension("ts"), Some(LanguageId::TypeScript));
        assert_eq!(LanguageId::from_extension("py"), Some(LanguageId::Python));
        assert_eq!(LanguageId::from_extension("go"), Some(LanguageId::Go));
        assert_eq!(LanguageId::from_extension("txt"), None);
    }
}
