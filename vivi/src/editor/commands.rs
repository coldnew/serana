use std::path::Path;

use crate::buffer::Buffer;
use crate::editor::Editor;

impl Editor {
    pub(super) fn execute_command(&mut self, cmd: &str) {
        let cmd = cmd.trim().trim_start_matches(':').trim();

        if cmd.is_empty() {
            return;
        }

        if self.execute_address(cmd) {
            return;
        }

        let (name, bang, arg) = split_ex_command(cmd);

        if name == "w" || name == "write" {
            self.write_buffer(arg);
        } else if name == "q" || name == "quit" {
            if self.buffer.is_modified() {
                if bang {
                    self.should_quit = true;
                } else {
                    self.message = Some("No write since last change (add ! to override)".into());
                }
            } else {
                self.should_quit = true;
            }
        } else if name == "wq" || name == "x" {
            self.write_buffer(arg);
            if !self
                .message
                .as_deref()
                .is_some_and(|msg| msg.starts_with("Error writing"))
            {
                self.should_quit = true;
            }
        } else if name == "e" || name == "edit" {
            self.edit_buffer(arg, bang);
        } else if cmd == "set number" || cmd == "set nu" {
            self.message = Some("number (always on)".into());
        } else if cmd == "set nonumber" || cmd == "set nonu" {
            self.message = Some("number (always on)".into());
        } else {
            self.message = Some(format!("Not an editor command: {}", cmd));
        }
    }

    fn execute_address(&mut self, cmd: &str) -> bool {
        let line = if cmd == "$" {
            Some(self.buffer.line_count())
        } else {
            cmd.parse::<usize>().ok()
        };

        let Some(line) = line else {
            return false;
        };

        if line == 0 || line > self.buffer.line_count() {
            self.message = Some(format!("Invalid range: {}", cmd));
            return true;
        }

        self.cursor.row = line - 1;
        self.cursor.col = 0;
        true
    }

    fn write_buffer(&mut self, path: &str) {
        let result = if path.is_empty() {
            self.buffer.save()
        } else {
            self.buffer.save_as(Path::new(path))
        };

        match result {
            Ok(()) => {
                let name = if path.is_empty() {
                    self.buffer.display_name()
                } else {
                    path.to_string()
                };
                self.message = Some(format!("\"{}\" written", name));
            }
            Err(e) => {
                self.message = Some(format!("Error writing: {}", e));
            }
        }
    }

    fn edit_buffer(&mut self, path: &str, bang: bool) {
        if self.buffer.is_modified() && !bang {
            self.message = Some("No write since last change (add ! to override)".into());
            return;
        }

        let path = if path.is_empty() {
            let Some(path) = self.buffer.path().map(Path::to_path_buf) else {
                self.message = Some("No file name".into());
                return;
            };
            path
        } else {
            Path::new(path).to_path_buf()
        };

        match Buffer::from_file(&path) {
            Ok(new_buf) => {
                let display = path.to_string_lossy().into_owned();
                *self = Editor::new(new_buf);
                self.message = Some(format!("Opened \"{}\"", display));
            }
            Err(e) => {
                self.message = Some(format!("Error opening file: {}", e));
            }
        }
    }
}

fn split_ex_command(cmd: &str) -> (&str, bool, &str) {
    let name = cmd.split_whitespace().next().unwrap_or("");
    let arg = cmd[name.len()..].trim_start();
    match name.strip_suffix('!') {
        Some(name) => (name, true, arg),
        None => (name, false, arg),
    }
}
