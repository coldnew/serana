use std::path::Path;

use crate::buffer::Buffer;
use crate::editor::Editor;

impl Editor {
    pub(super) fn execute_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();

        if cmd == "w" || cmd == "write" {
            match self.buffer.save() {
                Ok(()) => {
                    self.message = Some(format!("\"{}\" written", self.buffer.display_name()));
                }
                Err(e) => {
                    self.message = Some(format!("Error writing: {}", e));
                }
            }
        } else if cmd == "q" || cmd == "quit" {
            if self.buffer.is_modified() {
                self.message = Some("No write since last change (add ! to override)".into());
            } else {
                self.should_quit = true;
            }
        } else if cmd == "q!" || cmd == "quit!" {
            self.should_quit = true;
        } else if cmd == "wq" || cmd == "x" {
            match self.buffer.save() {
                Ok(()) => {
                    self.should_quit = true;
                }
                Err(e) => {
                    self.message = Some(format!("Error writing: {}", e));
                }
            }
        } else if let Some(rest) = cmd.strip_prefix("w ") {
            let path = Path::new(rest.trim());
            match self.buffer.save_as(path) {
                Ok(()) => {
                    self.message = Some(format!("\"{}\" written", rest.trim()));
                }
                Err(e) => {
                    self.message = Some(format!("Error writing: {}", e));
                }
            }
        } else if let Some(rest) = cmd.strip_prefix("e ") {
            let path = Path::new(rest.trim());
            match Buffer::from_file(path) {
                Ok(new_buf) => {
                    *self = Editor::new(new_buf);
                    self.message = Some(format!("Opened \"{}\"", rest.trim()));
                }
                Err(e) => {
                    self.message = Some(format!("Error opening file: {}", e));
                }
            }
        } else if cmd == "set number" || cmd == "set nu" {
            self.message = Some("number (always on)".into());
        } else if let Some(rest) = cmd.strip_prefix(':') {
            if let Ok(line) = rest.trim().parse::<usize>() {
                let line_count = self.buffer.line_count();
                if line > 0 && line <= line_count {
                    self.cursor.row = line - 1;
                    self.cursor.col = 0;
                }
            } else {
                self.message = Some(format!("Not an editor command: {}", cmd));
            }
        } else {
            self.message = Some(format!("Not an editor command: {}", cmd));
        }
    }
}
