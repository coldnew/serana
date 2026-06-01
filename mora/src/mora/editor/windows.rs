use super::{MoraEditor, WindowState};
use crate::mora::view::View;

impl MoraEditor {
    pub fn split_window_horizontal(&mut self) {
        self.sync_buffer_to_window();
        let height = self.view.height;
        let half = height / 2;
        if half < 3 {
            self.status_message = "Window too small to split".to_string();
            return;
        }
        let new_view = View::new(half);
        self.view.height = half;
        self.windows.push(WindowState {
            view: new_view,
            buffer_idx: self.current_window_buffer_idx,
            cursor: self.buffer.cursor,
        });
        self.current_window_idx = self.windows.len() - 1;
        self.sync_window_to_buffer();
        self.status_message = format!("Split horizontal ({} windows)", self.windows.len());
    }

    pub fn split_window_vertical(&mut self) {
        self.sync_buffer_to_window();
        let width = 80;
        let half = width / 2;
        if half < 10 {
            self.status_message = "Window too small to split".to_string();
            return;
        }
        let new_view = View::new(self.view.height);
        self.windows.push(WindowState {
            view: new_view,
            buffer_idx: self.current_window_buffer_idx,
            cursor: self.buffer.cursor,
        });
        self.current_window_idx = self.windows.len() - 1;
        self.sync_window_to_buffer();
        self.status_message = format!("Split vertical ({} windows)", self.windows.len());
    }

    pub(super) fn delete_window(&mut self) {
        if self.windows.len() <= 1 {
            self.status_message = "Can't delete last window".to_string();
            return;
        }
        self.sync_buffer_to_window();
        self.windows.remove(self.current_window_idx);
        if self.current_window_idx >= self.windows.len() {
            self.current_window_idx = self.windows.len() - 1;
        }
        self.sync_window_to_buffer();
        self.status_message = format!("Deleted window ({} remaining)", self.windows.len());
    }

    pub(super) fn delete_other_windows(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }
        self.sync_buffer_to_window();
        let current = self.windows[self.current_window_idx].clone();
        self.windows.clear();
        self.windows.push(current);
        self.current_window_idx = 0;
        self.sync_window_to_buffer();
        self.status_message = "Deleted other windows".to_string();
    }

    pub fn other_window(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }
        self.sync_buffer_to_window();
        self.current_window_idx = (self.current_window_idx + 1) % self.windows.len();
        self.sync_window_to_buffer();
        self.status_message = format!(
            "Window {}/{}",
            self.current_window_idx + 1,
            self.windows.len()
        );
    }

    pub(super) fn balance_windows(&mut self) {
        if self.windows.is_empty() {
            return;
        }
        let total_height = self.view.height * self.windows.len();
        let per_window = total_height / self.windows.len();
        for win in &mut self.windows {
            win.view.height = per_window;
        }
        self.status_message = "Balanced windows".to_string();
    }

    fn sync_buffer_to_window(&mut self) {
        if self.current_window_idx < self.windows.len() {
            self.windows[self.current_window_idx].cursor = self.buffer.cursor;
        }
    }

    fn sync_window_to_buffer(&mut self) {
        if self.current_window_idx < self.windows.len() {
            let win = &self.windows[self.current_window_idx];
            self.buffer.cursor = win.cursor;
        }
    }

    pub fn window_index_display(win: &WindowState) -> String {
        format!("[{}]", win.buffer_idx)
    }
}
