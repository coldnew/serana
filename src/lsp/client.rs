//! LSP client implementation

use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

use crate::Result;

/// LSP client for a single language server
pub struct LspClient {
    #[allow(dead_code)]
    process: Child,
    #[allow(dead_code)]
    workspace_root: std::path::PathBuf,
}

impl LspClient {
    /// Spawn a new language server process
    pub async fn spawn(command: &str, workspace_root: &Path) -> Result<Self> {
        let process = Command::new(command)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(Self {
            process,
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    /// Shutdown the language server
    pub async fn shutdown(mut self) -> Result<()> {
        self.process.kill().await?;
        Ok(())
    }

    // TODO: Implement LSP protocol methods:
    // - initialize
    // - textDocument/definition
    // - textDocument/references
    // - textDocument/hover
    // - textDocument/rename
    // - textDocument/publishDiagnostics
}
