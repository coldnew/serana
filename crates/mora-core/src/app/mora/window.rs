use super::buffer::Buffer;
use super::view::View;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct Window {
    pub buffer_idx: usize,
    pub view: View,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub is_active: bool,
}

impl Window {
    pub fn new(buffer_idx: usize, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            buffer_idx,
            view: View::new(height as usize),
            x,
            y,
            width,
            height,
            is_active: true,
        }
    }

    pub fn contains_point(&self, px: u16, py: u16) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

#[derive(Debug)]
pub struct WindowManager {
    pub windows: Vec<Window>,
    pub active_idx: usize,
    pub buffers: Vec<Buffer>,
}

impl WindowManager {
    pub fn new(width: u16, height: u16) -> Self {
        let mut buffers = Vec::new();
        buffers.push(Buffer::new());
        let window = Window::new(0, 0, 1, width, height.saturating_sub(2));
        Self {
            windows: vec![window],
            active_idx: 0,
            buffers,
        }
    }

    pub fn active_window(&self) -> &Window {
        &self.windows[self.active_idx]
    }

    pub fn active_window_mut(&mut self) -> &mut Window {
        &mut self.windows[self.active_idx]
    }

    pub fn active_buffer(&self) -> &Buffer {
        let idx = self.windows[self.active_idx].buffer_idx;
        &self.buffers[idx]
    }

    pub fn active_buffer_mut(&mut self) -> &mut Buffer {
        let idx = self.windows[self.active_idx].buffer_idx;
        &mut self.buffers[idx]
    }

    pub fn split_horizontal(&mut self) {
        let win = &self.windows[self.active_idx];
        let half_height = win.height / 2;
        if half_height < 3 {
            return;
        }

        let buffer_idx = win.buffer_idx;
        let x = win.x;
        let y = win.y;
        let width = win.width;

        let mut new_win = Window::new(buffer_idx, x, y + half_height, width, win.height - half_height);
        new_win.view.scroll_top = win.view.scroll_top;

        self.windows[self.active_idx].height = half_height;
        self.windows[self.active_idx].view.height = half_height as usize;

        self.windows.push(new_win);
        self.active_idx = self.windows.len() - 1;
    }

    pub fn split_vertical(&mut self) {
        let win = &self.windows[self.active_idx];
        let half_width = win.width / 2;
        if half_width < 10 {
            return;
        }

        let buffer_idx = win.buffer_idx;
        let x = win.x;
        let y = win.y;
        let height = win.height;

        let mut new_win = Window::new(buffer_idx, x + half_width, y, win.width - half_width, height);
        new_win.view.scroll_top = win.view.scroll_top;

        self.windows[self.active_idx].width = half_width;
        self.windows[self.active_idx].view.gutter_width = 4;

        self.windows.push(new_win);
        self.active_idx = self.windows.len() - 1;
    }

    pub fn delete_window(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }

        let closed = self.windows.remove(self.active_idx);
        if self.active_idx >= self.windows.len() {
            self.active_idx = self.windows.len() - 1;
        }

        let remaining_buffer_indices: Vec<usize> = self.windows.iter().map(|w| w.buffer_idx).collect();
        let closed_buffer_idx = closed.buffer_idx;
        if !remaining_buffer_indices.contains(&closed_buffer_idx) {
            if closed_buffer_idx < self.buffers.len() {
                self.buffers.remove(closed_buffer_idx);
                for w in &mut self.windows {
                    if w.buffer_idx > closed_buffer_idx {
                        w.buffer_idx -= 1;
                    }
                }
            }
        }

        self.balance_windows();
    }

    pub fn delete_other_windows(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }

        let active_buffer_idx = self.windows[self.active_idx].buffer_idx;
        let x = 0;
        let y = 1;
        let width = self.windows.iter().map(|w| w.x + w.width).max().unwrap_or(80);
        let height = self.windows.iter().map(|w| w.y + w.height).max().unwrap_or(24) - y;

        let remaining_buffer_indices: Vec<usize> = vec![active_buffer_idx];

        self.windows.retain(|w| w.buffer_idx == active_buffer_idx);
        self.windows[0].x = x;
        self.windows[0].y = y;
        self.windows[0].width = width;
        self.windows[0].height = height;
        self.windows[0].view.height = height as usize;
        self.active_idx = 0;

        let indices_to_remove: Vec<usize> = (0..self.buffers.len())
            .filter(|i| !remaining_buffer_indices.contains(i))
            .collect();
        for idx in indices_to_remove.into_iter().rev() {
            self.buffers.remove(idx);
            for w in &mut self.windows {
                if w.buffer_idx > idx {
                    w.buffer_idx -= 1;
                }
            }
        }
    }

    pub fn other_window(&mut self) {
        if self.windows.len() > 1 {
            self.active_idx = (self.active_idx + 1) % self.windows.len();
        }
    }

    pub fn balance_windows(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        let total_x = self.windows.iter().map(|w| w.x + w.width).max().unwrap_or(80);
        let total_y = self.windows.iter().map(|w| w.y + w.height).max().unwrap_or(24);
        let count = self.windows.len() as u16;

        if count == 1 {
            self.windows[0].x = 0;
            self.windows[0].y = 1;
            self.windows[0].width = total_x;
            self.windows[0].height = total_y.saturating_sub(1);
            self.windows[0].view.height = self.windows[0].height as usize;
            return;
        }

        let has_horizontal = self.windows.windows(2).any(|w| w[0].y != w[1].y);
        let has_vertical = self.windows.windows(2).any(|w| w[0].x != w[1].x);

        if has_horizontal && !has_vertical {
            let y_start = 1u16;
            let available_height = total_y.saturating_sub(y_start);
            let win_height = available_height / count;
            let remainder = available_height % count;

            for (i, win) in self.windows.iter_mut().enumerate() {
                win.x = 0;
                win.y = y_start + (i as u16) * win_height + (i as u16).min(remainder);
                win.width = total_x;
                win.height = win_height + if (i as u16) < remainder { 1 } else { 0 };
                win.view.height = win.height as usize;
            }
        } else if has_vertical && !has_horizontal {
            let x_start = 0u16;
            let available_width = total_x.saturating_sub(x_start);
            let win_width = available_width / count;
            let remainder = available_width % count;

            for (i, win) in self.windows.iter_mut().enumerate() {
                win.x = x_start + (i as u16) * win_width + (i as u16).min(remainder);
                win.y = 1;
                win.width = win_width + if (i as u16) < remainder { 1 } else { 0 };
                win.height = total_y.saturating_sub(1);
                win.view.height = win.height as usize;
            }
        }
    }

    pub fn open_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let buffer = Buffer::from_file(path)?;
        self.buffers.push(buffer);
        let new_idx = self.buffers.len() - 1;
        self.windows[self.active_idx].buffer_idx = new_idx;
        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.balance_windows();
        for win in &mut self.windows {
            win.width = (win.width * width) / (win.x + win.width).max(1);
            win.height = (win.height * height) / (win.y + win.height).max(1);
            win.view.height = win.height as usize;
        }
    }
}
