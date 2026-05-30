use std::path::PathBuf;
use std::process::Command;

/// Maximum number of output lines to keep in the shell buffer.
const MAX_OUTPUT_LINES: usize = 10000;
/// Maximum number of history entries.
const MAX_HISTORY: usize = 1000;

/// State for mshell — the in-editor shell (like Emacs eshell).
pub struct MshellState {
    /// Lines of shell output to display.
    pub output_lines: Vec<String>,
    /// Current input buffer.
    pub input: String,
    /// Cursor position within the input buffer.
    pub cursor: usize,
    /// Command history for up/down navigation.
    pub history: Vec<String>,
    /// Current position in history (index from end).
    pub history_pos: usize,
    /// Temporary input saved before browsing history.
    pub saved_input: String,
    /// Whether mshell is currently active.
    pub active: bool,
    /// Current working directory.
    pub working_dir: PathBuf,
}

impl MshellState {
    pub fn new() -> Self {
        Self {
            output_lines: Vec::new(),
            input: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: 0,
            saved_input: String::new(),
            active: false,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Start mshell mode.
    pub fn start(&mut self) {
        self.active = true;
        self.input.clear();
        self.cursor = 0;
        self.working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        if self.output_lines.is_empty() {
            self.output_lines.push("Welcome to Mshell. Type a command and press Enter.".to_string());
            self.output_lines.push(String::new());
        }
    }

    /// Cancel mshell mode, return to normal editing.
    pub fn cancel(&mut self) {
        self.active = false;
        self.input.clear();
        self.cursor = 0;
    }

    /// Get the shell prompt string.
    pub fn prompt(&self) -> String {
        let dir = self.working_dir.to_string_lossy();
        // Shorten home dir to ~
        let display = if let Some(home) = std::env::var_os("HOME") {
            let home = home.to_string_lossy();
            if dir.starts_with(&*home) {
                format!("~{}", &dir[home.len()..])
            } else {
                dir.into_owned()
            }
        } else {
            dir.into_owned()
        };
        format!("{} $ ", display)
    }

    /// Execute the current input as a shell command.
    pub fn execute(&mut self) {
        let input = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.history_pos = 0;
        self.saved_input.clear();

        let trimmed = input.trim();
        if trimmed.is_empty() {
            self.output_lines.push(self.prompt());
            return;
        }

        // Record in history
        if self.history.last().map(|s| s.as_str()) != Some(trimmed) {
            self.history.push(trimmed.to_string());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }

        // Show the command
        self.output_lines.push(format!("{}{}", self.prompt(), trimmed));

        // Handle cd specially
        if trimmed == "cd" || trimmed.starts_with("cd ") {
            self.handle_cd(trimmed);
            return;
        }

        // Handle exit
        if trimmed == "exit" {
            self.cancel();
            return;
        }

        // Execute via sh -c
        let output = Command::new("sh")
            .arg("-c")
            .arg(trimmed)
            .current_dir(&self.working_dir)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                for line in stdout.lines() {
                    self.output_lines.push(line.to_string());
                }
                if !stderr.is_empty() {
                    for line in stderr.lines() {
                        self.output_lines.push(format!("stderr: {}", line));
                    }
                }
            }
            Err(e) => {
                self.output_lines.push(format!("Error: {}", e));
            }
        }

        // Trim output if too large
        if self.output_lines.len() > MAX_OUTPUT_LINES {
            let excess = self.output_lines.len() - MAX_OUTPUT_LINES;
            self.output_lines.drain(0..excess);
        }
    }

    fn handle_cd(&mut self, input: &str) {
        let target = if input.trim() == "cd" {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"))
        } else {
            let path_str = input.trim_start_matches("cd").trim();
            let path = PathBuf::from(path_str);
            if path.is_absolute() {
                path
            } else {
                self.working_dir.join(path)
            }
        };

        // Canonicalize to resolve . and ..
        let target = target.canonicalize().unwrap_or(target);

        if target.is_dir() {
            self.working_dir = target;
            if let Err(e) = std::env::set_current_dir(&self.working_dir) {
                self.output_lines.push(format!("cd: {}", e));
            }
        } else {
            self.output_lines.push(format!("cd: no such directory: {}", input.trim_start_matches("cd").trim()));
        }
    }

    /// Handle a key event while mshell is active.
    /// Returns true if the event was consumed.
    pub fn handle_key(&mut self, key: &super::display::event::MoraKeyEvent) -> bool {
        use super::display::event::{MoraKeyCode as KC, MoraKeyModifiers as KM};

        match key.code {
            KC::Enter => {
                self.execute();
                true
            }
            KC::Esc => {
                // Ctrl+G equivalent — cancel
                self.cancel();
                true
            }
            KC::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                true
            }
            KC::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
                true
            }
            KC::Left => {
                if key.modifiers.ctrl {
                    // Ctrl+Left: move word backward
                    self.cursor = word_backward(&self.input, self.cursor);
                } else if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KC::Right => {
                if key.modifiers.ctrl {
                    // Ctrl+Right: move word forward
                    self.cursor = word_forward(&self.input, self.cursor);
                } else if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                true
            }
            KC::Home | KC::Up if key.modifiers.ctrl => {
                // Ctrl+A / Ctrl+Up: beginning of line
                self.cursor = 0;
                true
            }
            KC::Up => {
                // History up
                if !self.history.is_empty() {
                    if self.history_pos == 0 {
                        self.saved_input = self.input.clone();
                    }
                    if self.history_pos < self.history.len() {
                        self.history_pos += 1;
                        let idx = self.history.len() - self.history_pos;
                        self.input = self.history[idx].clone();
                        self.cursor = self.input.len();
                    }
                }
                true
            }
            KC::Down => {
                // History down
                if self.history_pos > 0 {
                    self.history_pos -= 1;
                    if self.history_pos == 0 {
                        self.input = std::mem::take(&mut self.saved_input);
                    } else {
                        let idx = self.history.len() - self.history_pos;
                        self.input = self.history[idx].clone();
                    }
                    self.cursor = self.input.len();
                }
                true
            }
            KC::Char(c) => {
                if key.modifiers.ctrl {
                    match c {
                        'c' => {
                            self.cancel();
                            true
                        }
                        'a' => {
                            self.cursor = 0;
                            true
                        }
                        'e' => {
                            self.cursor = self.input.len();
                            true
                        }
                        'u' => {
                            // Kill from cursor to beginning
                            self.input.drain(..self.cursor);
                            self.cursor = 0;
                            true
                        }
                        'k' => {
                            // Kill from cursor to end
                            self.input.truncate(self.cursor);
                            true
                        }
                        'w' => {
                            // Kill word backward
                            let new_pos = word_backward(&self.input, self.cursor);
                            self.input.drain(new_pos..self.cursor);
                            self.cursor = new_pos;
                            true
                        }
                        'l' => {
                            // Clear output
                            self.output_lines.clear();
                            true
                        }
                        _ => false,
                    }
                } else if key.modifiers.alt {
                    false
                } else {
                    self.input.insert(self.cursor, c);
                    self.cursor += 1;
                    true
                }
            }
            KC::Tab => {
                // Insert spaces for tab
                self.input.insert_str(self.cursor, "    ");
                self.cursor += 4;
                true
            }
            _ => false,
        }
    }
}

fn word_backward(s: &str, pos: usize) -> usize {
    let bytes = s.as_bytes();
    let mut i = pos;
    // Skip whitespace
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    // Skip word chars
    while i > 0 && !bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    i
}

fn word_forward(s: &str, pos: usize) -> usize {
    let bytes = s.as_bytes();
    let mut i = pos;
    // Skip non-whitespace
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    // Skip whitespace
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
