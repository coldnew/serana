use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};

// ── Winner ring (thread-local window config stack) ───────────

thread_local! {
    static WINNER_RING: RefCell<VecDeque<WinnerEntry>> = RefCell::new(VecDeque::new());
    static WINNER_REDO_RING: RefCell<VecDeque<WinnerEntry>> = RefCell::new(VecDeque::new());
}

struct WinnerEntry {
    cursor_row: usize,
    cursor_col: usize,
    window_count: usize,
}

const WINNER_MAX: usize = 20;

fn winner_push_entry(entry: WinnerEntry) {
    WINNER_RING.with(|ring| {
        let mut ring = ring.borrow_mut();
        if ring.len() >= WINNER_MAX {
            ring.pop_back();
        }
        ring.push_front(entry);
    });
    // Clear redo ring on new push
    WINNER_REDO_RING.with(|ring| ring.borrow_mut().clear());
}

fn winner_pop_entry() -> Option<WinnerEntry> {
    WINNER_RING.with(|ring| ring.borrow_mut().pop_front())
}

fn winner_push_redo_entry(entry: WinnerEntry) {
    WINNER_REDO_RING.with(|ring| {
        ring.borrow_mut().push_front(entry);
    });
}

fn winner_pop_redo_entry() -> Option<WinnerEntry> {
    WINNER_REDO_RING.with(|ring| ring.borrow_mut().pop_front())
}

// ── Helpers ──────────────────────────────────────────────────

/// Extract word before cursor on the current line.
fn word_before_cursor(lines: &[String], row: usize, col: usize) -> String {
    if row >= lines.len() {
        return String::new();
    }
    let line = &lines[row];
    let col = col.min(line.len());
    let before = &line[..col];
    // Find the start of the word (alphanumeric + underscore)
    let start = before.rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);
    before[start..].to_string()
}

/// Find all words in lines matching prefix, excluding the exact exclude word.
fn find_matching_words(lines: &[String], prefix: &str, exclude: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut matches = Vec::new();
    for line in lines {
        let mut chars = line.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if i == 0 || !line.as_bytes().get(i.wrapping_sub(1)).map_or(false, |b| b.is_ascii_alphanumeric() || *b == b'_') {
                if c.is_alphanumeric() || c == '_' {
                    let word_start = i;
                    let mut word_end = i + c.len_utf8();
                    while let Some(&(j, next_c)) = chars.peek() {
                        if next_c.is_alphanumeric() || next_c == '_' {
                            word_end = j + next_c.len_utf8();
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let word = &line[word_start..word_end];
                    if word.starts_with(prefix) && word != exclude && seen.insert(word.to_string()) {
                        matches.push(word.to_string());
                    }
                }
            }
        }
    }
    matches
}

/// Convert (row, col) to linear byte offset in lines.
fn pos_to_offset(lines: &[String], row: usize, col: usize) -> usize {
    let mut offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i == row {
            return offset + col.min(line.len());
        }
        offset += line.len() + 1; // +1 for newline
    }
    offset
}

/// Convert linear byte offset to (row, col) in lines.
fn offset_to_pos(lines: &[String], offset: usize) -> (usize, usize) {
    let mut remaining = offset;
    for (i, line) in lines.iter().enumerate() {
        if remaining <= line.len() {
            return (i, remaining);
        }
        remaining -= line.len() + 1;
    }
    let last = lines.len().saturating_sub(1);
    (last, lines.last().map_or(0, |l| l.len()))
}

/// Determine comment prefix based on file extension.
fn comment_prefix(file_path: &Option<String>) -> &'static str {
    if let Some(path) = file_path {
        if path.ends_with(".lisp") || path.ends_with(".lsp") || path.ends_with(".cl")
            || path.ends_with(".el") || path.ends_with(".scm") || path.ends_with(".mora") {
            return "; ";
        }
        if path.ends_with(".hs") {
            return "-- ";
        }
        if path.ends_with(".py") || path.ends_with(".rb") || path.ends_with(".sh")
            || path.ends_with(".bash") || path.ends_with(".zsh") || path.ends_with(".yml")
            || path.ends_with(".yaml") || path.ends_with(".toml") {
            return "# ";
        }
    }
    "; "
}

/// Create a 2-element vector [row, col].
fn pos_value(row: usize, col: usize) -> Value {
    Value::Vector(Arc::new(vec![Value::Int(row as i64), Value::Int(col as i64)]))
}

// ── Dabbrev expand ──────────────────────────────────────────

/// (dabbrev-expand) — Expand word at cursor to match a word elsewhere in buffer.
fn prim_dabbrev_expand(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let prefix = word_before_cursor(&state.lines, state.cursor_row, state.cursor_col);
        if prefix.is_empty() {
            return Err("No word before cursor".to_string());
        }

        // Check if this is a continuation (same prefix as before)
        if state.dabbrev_prefix == prefix && !state.dabbrev_matches.is_empty() {
            // Cycle to next match
            state.dabbrev_index = (state.dabbrev_index + 1) % state.dabbrev_matches.len();
            let replacement = state.dabbrev_matches[state.dabbrev_index].clone();

            // Replace word on current line
            let col = state.cursor_col;
            let word_start = col.saturating_sub(prefix.len());
            state.lines[state.cursor_row].replace_range(word_start..col, &replacement);
            state.cursor_col = word_start + replacement.len();
            state.modified = true;

            return Ok(Value::string(replacement));
        }

        // First call: find all matches
        let matches = find_matching_words(&state.lines, &prefix, &prefix);
        if matches.is_empty() {
            return Err(format!("No expansion found for \"{}\"", prefix));
        }

        let replacement = matches[0].clone();
        let prefix_len = prefix.len();
        state.dabbrev_prefix = prefix;
        state.dabbrev_matches = matches;
        state.dabbrev_index = 0;

        // Replace word on current line
        let col = state.cursor_col;
        let actual_start = col - prefix_len;
        state.lines[state.cursor_row].replace_range(actual_start..col, &replacement);
        state.cursor_col = actual_start + replacement.len();
        state.modified = true;

        Ok(Value::string(replacement))
    })
}

/// (dabbrev-reset) — Reset dabbrev state.
fn prim_dabbrev_reset(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.dabbrev_prefix.clear();
        state.dabbrev_matches.clear();
        state.dabbrev_index = 0;
        Ok(Value::Nil)
    })
}

// ── Symbol overlay ──────────────────────────────────────────

/// (symbol-overlay-highlight) — Highlight all occurrences of word at cursor.
fn prim_symbol_overlay_highlight(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let word = word_before_cursor(&state.lines, state.cursor_row, state.cursor_col);
        if word.is_empty() {
            return Err("No word at cursor".to_string());
        }

        // Remove existing symbol overlays
        state.overlays.clear_by_category("symbol-overlay");

        // Scan all lines for matches and collect positions
        let mut positions: Vec<Value> = Vec::new();
        for (row, line) in state.lines.iter().enumerate() {
            let mut search_start = 0;
            while let Some(col) = line[search_start..].find(&word) {
                let abs_col = search_start + col;
                // Verify word boundary
                let before_ok = abs_col == 0 || !line.as_bytes()[abs_col - 1].is_ascii_alphanumeric() && line.as_bytes()[abs_col - 1] != b'_';
                let end = abs_col + word.len();
                let after_ok = end >= line.len() || !line.as_bytes()[end].is_ascii_alphanumeric() && line.as_bytes()[end] != b'_';

                if before_ok && after_ok {
                    let start_offset = pos_to_offset(&state.lines, row, abs_col);
                    let end_offset = pos_to_offset(&state.lines, row, end);
                    let id = state.overlays.add(start_offset, end_offset);
                    if let Some(ov) = state.overlays.get_mut(id) {
                        ov.category = Some("symbol-overlay".to_string());
                        ov.properties.insert("symbol".to_string(), word.clone());
                    }
                    positions.push(pos_value(row, abs_col));
                }
                search_start = abs_col + word.len();
            }
        }

        if positions.is_empty() {
            return Err(format!("No occurrences of \"{}\" found", word));
        }

        Ok(Value::Vector(Arc::new(positions)))
    })
}

/// (symbol-overlay-remove) — Remove all symbol overlays.
fn prim_symbol_overlay_remove(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.overlays.clear_by_category("symbol-overlay");
        Ok(Value::Nil)
    })
}

/// (symbol-overlay-jump-next) — Jump to next highlighted symbol.
fn prim_symbol_overlay_jump_next(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let cursor_offset = pos_to_offset(&state.lines, state.cursor_row, state.cursor_col);

        // Find all symbol overlays sorted by position
        let mut overlays: Vec<(usize, usize)> = Vec::new();
        for ov in state.overlays.all() {
            if ov.category.as_deref() == Some("symbol-overlay") {
                overlays.push((ov.start, ov.end));
            }
        }
        overlays.sort_by_key(|(s, _)| *s);

        // Find next overlay strictly after cursor
        for &(start, _) in &overlays {
            if start > cursor_offset {
                let (row, col) = offset_to_pos(&state.lines, start);
                state.cursor_row = row;
                state.cursor_col = col;
                return Ok(pos_value(row, col));
            }
        }

        // Wrap around to first
        if let Some(&(start, _)) = overlays.first() {
            let (row, col) = offset_to_pos(&state.lines, start);
            state.cursor_row = row;
            state.cursor_col = col;
            return Ok(pos_value(row, col));
        }

        Err("No symbol overlays to jump to".to_string())
    })
}

/// (symbol-overlay-jump-prev) — Jump to previous highlighted symbol.
fn prim_symbol_overlay_jump_prev(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let cursor_offset = pos_to_offset(&state.lines, state.cursor_row, state.cursor_col);

        let mut overlays: Vec<(usize, usize)> = Vec::new();
        for ov in state.overlays.all() {
            if ov.category.as_deref() == Some("symbol-overlay") {
                overlays.push((ov.start, ov.end));
            }
        }
        overlays.sort_by_key(|(s, _)| *s);

        // Find previous overlay strictly before cursor
        for &(start, _) in overlays.iter().rev() {
            if start < cursor_offset {
                let (row, col) = offset_to_pos(&state.lines, start);
                state.cursor_row = row;
                state.cursor_col = col;
                return Ok(pos_value(row, col));
            }
        }

        // Wrap around to last
        if let Some(&(start, _)) = overlays.last() {
            let (row, col) = offset_to_pos(&state.lines, start);
            state.cursor_row = row;
            state.cursor_col = col;
            return Ok(pos_value(row, col));
        }

        Err("No symbol overlays to jump to".to_string())
    })
}

// ── Winner mode ─────────────────────────────────────────────

/// (winner-push) — Push current window config onto winner ring.
fn prim_winner_push(_args: &[Value]) -> Result<Value, String> {
    let (row, col, wc) = with_editor_state(|state| {
        (state.cursor_row, state.cursor_col, state.window_count)
    });
    winner_push_entry(WinnerEntry {
        cursor_row: row,
        cursor_col: col,
        window_count: wc,
    });
    Ok(Value::Nil)
}

/// (winner-undo) — Restore previous window config.
fn prim_winner_undo(_args: &[Value]) -> Result<Value, String> {
    // Push current state onto redo ring first
    let (row, col, wc) = with_editor_state(|state| {
        (state.cursor_row, state.cursor_col, state.window_count)
    });
    winner_push_redo_entry(WinnerEntry {
        cursor_row: row,
        cursor_col: col,
        window_count: wc,
    });

    match winner_pop_entry() {
        Some(entry) => {
            with_editor_state_mut(|state| {
                state.cursor_row = entry.cursor_row;
                state.cursor_col = entry.cursor_col;
                state.window_count = entry.window_count;
            });
            Ok(Value::Bool(true))
        }
        None => Err("No window configuration to undo".to_string()),
    }
}

/// (winner-redo) — Re-apply next window config.
fn prim_winner_redo(_args: &[Value]) -> Result<Value, String> {
    // Push current state onto undo ring first
    let (row, col, wc) = with_editor_state(|state| {
        (state.cursor_row, state.cursor_col, state.window_count)
    });
    winner_push_entry(WinnerEntry {
        cursor_row: row,
        cursor_col: col,
        window_count: wc,
    });

    match winner_pop_redo_entry() {
        Some(entry) => {
            with_editor_state_mut(|state| {
                state.cursor_row = entry.cursor_row;
                state.cursor_col = entry.cursor_col;
                state.window_count = entry.window_count;
            });
            Ok(Value::Bool(true))
        }
        None => Err("No window configuration to redo".to_string()),
    }
}

// ── Comment operations ──────────────────────────────────────

/// (comment-region) — Comment out region between mark and cursor.
fn prim_comment_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let mark = state.mark_pos.ok_or("No mark set")?;
        let (start_row, end_row) = if mark.0 <= state.cursor_row {
            (mark.0, state.cursor_row)
        } else {
            (state.cursor_row, mark.0)
        };

        let prefix = comment_prefix(&state.file_path);

        for row in start_row..=end_row {
            if row < state.lines.len() {
                state.lines[row].insert_str(0, prefix);
            }
        }
        state.modified = true;
        Ok(Value::Bool(true))
    })
}

/// (uncomment-region) — Uncomment region between mark and cursor.
fn prim_uncomment_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let mark = state.mark_pos.ok_or("No mark set")?;
        let (start_row, end_row) = if mark.0 <= state.cursor_row {
            (mark.0, state.cursor_row)
        } else {
            (state.cursor_row, mark.0)
        };

        let prefixes: &[&str] = &["; ", "// ", "-- ", "# "];

        for row in start_row..=end_row {
            if row < state.lines.len() {
                let line = &mut state.lines[row];
                let trimmed_start = line.len() - line.trim_start().len();
                for &pfx in prefixes {
                    if line[trimmed_start..].starts_with(pfx) {
                        line.replace_range(trimmed_start..trimmed_start + pfx.len(), "");
                        break;
                    }
                }
            }
        }
        state.modified = true;
        Ok(Value::Bool(true))
    })
}

/// (comment-toggle) — Toggle comment on region.
fn prim_comment_toggle(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let mark = state.mark_pos.ok_or("No mark set")?;
        let (start_row, end_row) = if mark.0 <= state.cursor_row {
            (mark.0, state.cursor_row)
        } else {
            (state.cursor_row, mark.0)
        };

        let prefixes: &[&str] = &["; ", "// ", "-- ", "# "];

        // Check if any line is commented
        let any_commented = (start_row..=end_row).any(|row| {
            if row >= state.lines.len() {
                return false;
            }
            let trimmed = state.lines[row].trim_start();
            prefixes.iter().any(|pfx| trimmed.starts_with(pfx))
        });

        if any_commented {
            // Uncomment all
            for row in start_row..=end_row {
                if row < state.lines.len() {
                    let line = &mut state.lines[row];
                    let trimmed_start = line.len() - line.trim_start().len();
                    for &pfx in prefixes {
                        if line[trimmed_start..].starts_with(pfx) {
                            line.replace_range(trimmed_start..trimmed_start + pfx.len(), "");
                            break;
                        }
                    }
                }
            }
        } else {
            // Comment all
            let prefix = comment_prefix(&state.file_path);
            for row in start_row..=end_row {
                if row < state.lines.len() {
                    state.lines[row].insert_str(0, prefix);
                }
            }
        }

        state.modified = true;
        Ok(Value::Bool(true))
    })
}

// ── Rectangle operations ────────────────────────────────────

// Rectangle kill storage
thread_local! {
    static RECT_KILL: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Get column range from mark and cursor, normalized.
fn rect_col_range(state: &super::editor_state::EditorState) -> Result<(usize, usize, usize, usize), String> {
    let mark = state.mark_pos.ok_or("No mark set")?;
    let start_row = mark.0.min(state.cursor_row);
    let end_row = mark.0.max(state.cursor_row);
    let start_col = mark.1.min(state.cursor_col);
    let end_col = mark.1.max(state.cursor_col);
    Ok((start_row, end_row, start_col, end_col))
}

/// (rectangle-kill) — Kill text in rectangle between mark and cursor.
fn prim_rectangle_kill(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let (start_row, end_row, start_col, end_col) = rect_col_range(state)?;

        let mut killed = Vec::new();
        for row in start_row..=end_row {
            if row < state.lines.len() {
                let line = &mut state.lines[row];
                let sc = start_col.min(line.len());
                let ec = end_col.min(line.len());
                let extracted = line[sc..ec].to_string();
                line.replace_range(sc..ec, "");
                killed.push(extracted);
            } else {
                killed.push(String::new());
            }
        }

        RECT_KILL.with(|rk| {
            *rk.borrow_mut() = killed;
        });

        state.modified = true;
        Ok(Value::Bool(true))
    })
}

/// (rectangle-yank) — Yank killed rectangle at cursor.
fn prim_rectangle_yank(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let killed = RECT_KILL.with(|rk| rk.borrow().clone());
        if killed.is_empty() {
            return Err("No rectangle in kill ring".to_string());
        }

        let col = state.cursor_col;
        for (i, text) in killed.iter().enumerate() {
            let row = state.cursor_row + i;
            if row >= state.lines.len() {
                state.lines.resize(row + 1, String::new());
            }
            let line = &mut state.lines[row];
            let insert_col = col.min(line.len());
            line.insert_str(insert_col, text);
        }

        state.modified = true;
        Ok(Value::Bool(true))
    })
}

/// (rectangle-copy) — Copy rectangle to kill ring without deleting.
fn prim_rectangle_copy(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let (start_row, end_row, start_col, end_col) = rect_col_range(state)?;

        let mut copied = Vec::new();
        for row in start_row..=end_row {
            if row < state.lines.len() {
                let line = &state.lines[row];
                let sc = start_col.min(line.len());
                let ec = end_col.min(line.len());
                copied.push(line[sc..ec].to_string());
            } else {
                copied.push(String::new());
            }
        }

        RECT_KILL.with(|rk| {
            *rk.borrow_mut() = copied;
        });

        Ok(Value::Bool(true))
    })
}

// ── Registration ────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    // dabbrev
    ns.intern_with_doc("dabbrev-expand", Value::Native(prim_dabbrev_expand),
        "Expand word at cursor to match a word elsewhere in buffer. Repeat to cycle through matches.");
    ns.intern_private_with_doc("expand", Value::Native(prim_dabbrev_expand),
        "Expand word at cursor.");
    ns.intern_with_doc("dabbrev-reset", Value::Native(prim_dabbrev_reset),
        "Reset dabbrev expansion state.");
    ns.intern_private_with_doc("reset", Value::Native(prim_dabbrev_reset),
        "Reset dabbrev state.");

    // symbol-overlay
    ns.intern_with_doc("symbol-overlay-highlight", Value::Native(prim_symbol_overlay_highlight),
        "Highlight all occurrences of word at cursor.");
    ns.intern_private_with_doc("highlight", Value::Native(prim_symbol_overlay_highlight),
        "Highlight all occurrences of word at cursor.");
    ns.intern_with_doc("symbol-overlay-remove", Value::Native(prim_symbol_overlay_remove),
        "Remove all symbol overlay highlights.");
    ns.intern_private_with_doc("remove", Value::Native(prim_symbol_overlay_remove),
        "Remove symbol overlays.");
    ns.intern_with_doc("symbol-overlay-jump-next", Value::Native(prim_symbol_overlay_jump_next),
        "Jump to next highlighted symbol.");
    ns.intern_private_with_doc("jump-next", Value::Native(prim_symbol_overlay_jump_next),
        "Jump to next symbol.");
    ns.intern_with_doc("symbol-overlay-jump-prev", Value::Native(prim_symbol_overlay_jump_prev),
        "Jump to previous highlighted symbol.");
    ns.intern_private_with_doc("jump-prev", Value::Native(prim_symbol_overlay_jump_prev),
        "Jump to previous symbol.");

    // winner-mode
    ns.intern_with_doc("winner-push", Value::Native(prim_winner_push),
        "Push current window configuration onto winner ring.");
    ns.intern_private_with_doc("push", Value::Native(prim_winner_push),
        "Push window config.");
    ns.intern_with_doc("winner-undo", Value::Native(prim_winner_undo),
        "Restore previous window configuration from winner ring.");
    ns.intern_private_with_doc("undo", Value::Native(prim_winner_undo),
        "Undo window config.");
    ns.intern_with_doc("winner-redo", Value::Native(prim_winner_redo),
        "Re-apply next window configuration from winner ring.");
    ns.intern_private_with_doc("redo", Value::Native(prim_winner_redo),
        "Redo window config.");

    // comment-dwim
    ns.intern_with_doc("comment-region", Value::Native(prim_comment_region),
        "Comment out lines between mark and cursor.");
    ns.intern_private_with_doc("region", Value::Native(prim_comment_region),
        "Comment out region.");
    ns.intern_with_doc("uncomment-region", Value::Native(prim_uncomment_region),
        "Remove comment prefix from lines between mark and cursor.");
    ns.intern_private_with_doc("uncomment", Value::Native(prim_uncomment_region),
        "Uncomment region.");
    ns.intern_with_doc("comment-toggle", Value::Native(prim_comment_toggle),
        "Toggle comment on region: comment if any line is uncommented, uncomment if all are commented.");
    ns.intern_private_with_doc("toggle", Value::Native(prim_comment_toggle),
        "Toggle comment on region.");

    // rectangle
    ns.intern_with_doc("rectangle-kill", Value::Native(prim_rectangle_kill),
        "Kill text in rectangle between mark and cursor.");
    ns.intern_private_with_doc("kill", Value::Native(prim_rectangle_kill),
        "Kill rectangle.");
    ns.intern_with_doc("rectangle-yank", Value::Native(prim_rectangle_yank),
        "Yank killed rectangle at cursor position.");
    ns.intern_private_with_doc("yank", Value::Native(prim_rectangle_yank),
        "Yank rectangle.");
    ns.intern_with_doc("rectangle-copy", Value::Native(prim_rectangle_copy),
        "Copy rectangle to kill ring without deleting.");
    ns.intern_private_with_doc("copy", Value::Native(prim_rectangle_copy),
        "Copy rectangle.");
}
