use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use super::editor::MoraEditor;
use super::keymap::EditorMode;
use super::status_line;

pub struct EditorWidget<'a> {
    editor: &'a MoraEditor,
}

impl<'a> EditorWidget<'a> {
    pub fn new(editor: &'a MoraEditor) -> Self {
        Self { editor }
    }
}

impl<'a> Widget for EditorWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let editor = self.editor;

        let help_height = 1u16;
        let status_height = 1u16;
        let cmd_height: u16 = if matches!(
            editor.mode(),
            EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward
        ) {
            1
        } else {
            0
        };

        let editor_height = area
            .height
            .saturating_sub(help_height + status_height + cmd_height);

        let help_area = Rect::new(area.x, area.y, area.width, help_height);
        let editor_area = Rect::new(area.x, area.y + help_height, area.width, editor_height);
        let status_area = Rect::new(
            area.x,
            area.y + help_height + editor_height,
            area.width,
            status_height,
        );
        let cmd_area = Rect::new(
            area.x,
            area.y + help_height + editor_height + status_height,
            area.width,
            cmd_height,
        );

        let help = status_line::render_help_bar(editor.mode(), area.width as usize);
        buf.set_line(help_area.x, help_area.y, &help, help_area.width);

        render_editor_area(editor, editor_area, buf);

        let status = status_line::render_status_line(editor, area.width as usize);
        buf.set_line(status_area.x, status_area.y, &status, status_area.width);

        if cmd_height > 0 {
            let cmd = status_line::render_command_line(editor, area.width as usize);
            buf.set_line(cmd_area.x, cmd_area.y, &cmd, cmd_area.width);
        }
    }
}

fn render_editor_area(editor: &MoraEditor, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let view = editor.view();
    let text_buf = editor.buffer();
    let gutter_w = view.gutter_width;
    let text_width = area.width.saturating_sub(gutter_w);
    let narrow_start = text_buf.narrow_start.unwrap_or(0);
    let narrow_end = text_buf.narrow_end.unwrap_or(text_buf.line_count().saturating_sub(1));
    let total = text_buf.line_count();
    let (vis_start, vis_end) = view.visible_range(total);
    let render_start = vis_start.max(narrow_start);
    let render_end = vis_end.min(narrow_end + 1);

    let gutter_style = Style::new().fg(Color::Rgb(107, 114, 128));
    let text_style = Style::new().fg(Color::Rgb(232, 236, 244));
    let current_line_style = Style::new()
        .fg(Color::Rgb(0, 180, 255))
        .add_modifier(Modifier::BOLD);
    let cursor_style = Style::new()
        .fg(Color::Rgb(15, 18, 22))
        .bg(Color::Rgb(0, 180, 255));
    let selection_style = Style::new()
        .fg(Color::Rgb(232, 236, 244))
        .bg(Color::Rgb(30, 80, 120));

    let in_visual = editor.mode() == EditorMode::Visual && editor.mark_ring.is_active();
    let (sel_start, sel_end) = if in_visual {
        if let Some(mark) = editor.mark_ring.peek() {
            let cursor = text_buf.cursor;
            let (a, b) = if mark.row < cursor.row || (mark.row == cursor.row && mark.col <= cursor.col) {
                (*mark, cursor)
            } else {
                (cursor, *mark)
            };
            (Some(a), Some(b))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let mut display_row: u16 = 0;
    for line_idx in render_start..render_end {
        // Skip folded lines
        if text_buf.is_line_folded(line_idx) {
            continue;
        }

        let y = area.y + display_row;
        if y >= area.y + area.height {
            break;
        }

        let is_current = line_idx == text_buf.cursor.row;

        let line_num = if is_current {
            format!("{:>width$}", line_idx + 1, width = gutter_w as usize - 1)
        } else {
            format!(
                "{:>width$} ",
                line_idx + 1,
                width = gutter_w as usize - 1
            )
        };
        let gutter_style_actual = if is_current {
            current_line_style
        } else {
            gutter_style
        };

        for (col, ch) in line_num.chars().enumerate() {
            if col as u16 >= gutter_w {
                break;
            }
            buf[(area.x + col as u16, y)]
                .set_char(ch)
                .set_style(gutter_style_actual);
        }

        let line_text = text_buf.line(line_idx);
        let text_x = area.x + gutter_w;

        let mut byte_col = 0;
        let mut display_col: u16 = 0;
        for ch in line_text.chars() {
            if display_col >= text_width {
                break;
            }
            let x = text_x + display_col;

            let is_cursor =
                is_current && byte_col == text_buf.cursor.col && editor.mode() != EditorMode::Normal;

            let in_sel = sel_start.map_or(false, |start| {
                sel_end.map_or(false, |end| {
                    line_idx > start.row && line_idx < end.row
                        || (line_idx == start.row && line_idx == end.row
                            && byte_col >= start.col && byte_col < end.col)
                        || (line_idx == start.row && byte_col >= start.col)
                        || (line_idx == end.row && byte_col < end.col)
                })
            });

            let style = if is_cursor {
                cursor_style
            } else if in_sel {
                selection_style
            } else if is_current {
                current_line_style
            } else {
                text_style
            };

            if ch == '\t' {
                let tab_stop = 4 - (display_col % 4);
                for _ in 0..tab_stop {
                    if display_col >= text_width {
                        break;
                    }
                    buf[(text_x + display_col, y)]
                        .set_char(' ')
                        .set_style(style);
                    display_col += 1;
                }
            } else {
                buf[(x, y)].set_char(ch).set_style(style);
                display_col += 1;
            }
            byte_col += ch.len_utf8();
        }

        if is_current && editor.mode() == EditorMode::Normal {
            let cursor_display_col = line_text[..text_buf.cursor.col.min(line_text.len())]
                .chars()
                .count() as u16;
            if cursor_display_col < text_width {
                let x = text_x + cursor_display_col;
                let ch = line_text
                    .chars()
                    .nth(text_buf.cursor.col.min(line_text.len().saturating_sub(1)))
                    .unwrap_or(' ');
                buf[(x, y)].set_char(ch).set_style(cursor_style);
            }
        }

        while display_col < text_width {
            let x = text_x + display_col;
            let fill_sel = sel_start.map_or(false, |start| {
                sel_end.map_or(false, |end| {
                    line_idx > start.row && line_idx < end.row
                })
            });
            buf[(x, y)]
                .set_char(' ')
                .set_style(if fill_sel {
                    selection_style
                } else if is_current {
                    Style::new().bg(Color::Rgb(25, 28, 35))
                } else {
                    Style::default()
                });
            display_col += 1;
        }

        display_row += 1;
    }

    for view_row in display_row..area.height {
        let y = area.y + view_row;
        if y >= area.y + area.height {
            break;
        }
        let tilde_style = Style::new().fg(Color::Rgb(55, 60, 72));
        buf[(area.x, y)].set_char('~').set_style(tilde_style);
        for col in 1..area.width {
            buf[(area.x + col, y)].set_char(' ');
        }
    }
}
