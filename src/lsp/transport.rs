//! LSP transport layer for stdio communication

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout};

use crate::Result;

/// LSP transport for stdio communication
pub struct LspTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl LspTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    /// Send a JSON-RPC message to the language server
    pub fn send(&mut self, message: &str) -> Result<()> {
        let content_length = message.len();
        write!(
            self.stdin,
            "Content-Length: {}\r\n\r\n{}",
            content_length, message
        )?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Receive a JSON-RPC message from the language server
    pub fn receive(&mut self) -> Result<String> {
        // Read headers
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line)?;
            let line = line.trim();

            if line.is_empty() {
                break;
            }

            if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                content_length = len_str.parse()?;
            }
        }

        // Read content
        let mut buffer = vec![0u8; content_length];
        std::io::Read::read_exact(&mut self.stdout, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
