use crate::lisp::types::Value;
use super::editor_state::with_editor_state_mut;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    // expand-region
    ns.intern_with_doc("expand-region", Value::Native(prim_expand_region),
        "Expand selection by semantic units. Each call expands further: word → line → paragraph → buffer.");
    ns.intern_private_with_doc("expand", Value::Native(prim_expand_region),
        "Expand selection by semantic units.");
    ns.intern_with_doc("contract-region", Value::Native(prim_contract_region),
        "Contract selection back one level.");
    ns.intern_private_with_doc("contract", Value::Native(prim_contract_region),
        "Contract selection back one level.");

    // goto-last-change
    ns.intern_with_doc("goto-last-change", Value::Native(prim_goto_last_change),
        "Jump to last edit position using edit history.");
    ns.intern_private_with_doc("last-change", Value::Native(prim_goto_last_change),
        "Jump to last edit position.");

    // hungry-delete
    ns.intern_with_doc("hungry-delete-backward", Value::Native(prim_hungry_delete_backward),
        "Delete all whitespace backward until non-whitespace.");
    ns.intern_private_with_doc("hungry-back", Value::Native(prim_hungry_delete_backward),
        "Delete all whitespace backward.");
    ns.intern_with_doc("hungry-delete-forward", Value::Native(prim_hungry_delete_forward),
        "Delete all whitespace forward until non-whitespace.");
    ns.intern_private_with_doc("hungry-forward", Value::Native(prim_hungry_delete_forward),
        "Delete all whitespace forward.");

    // cleanup-buffer
    ns.intern_with_doc("cleanup-buffer", Value::Native(prim_cleanup_buffer),
        "Delete trailing whitespace, untabify the buffer.");
    ns.intern_private_with_doc("cleanup", Value::Native(prim_cleanup_buffer),
        "Delete trailing whitespace, untabify.");

    // insert-empty-line
    ns.intern_with_doc("insert-empty-line", Value::Native(prim_insert_empty_line),
        "Insert empty line after current line, move cursor there.");
    ns.intern_private_with_doc("empty-line", Value::Native(prim_insert_empty_line),
        "Insert empty line after current line.");

    // copy-and-comment
    ns.intern_with_doc("copy-and-comment", Value::Native(prim_copy_and_comment),
        "Copy region text to kill ring and comment out original lines.");
    ns.intern_private_with_doc("copy-comment", Value::Native(prim_copy_and_comment),
        "Copy region text and comment out original.");
}

/// (expand-region) — Expand selection by semantic units.
fn prim_expand_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let level = state.expand_region_level;

        match level {
            0 => {
                // Mark at cursor, expand to current word boundaries
                state.mark_active = true;
                state.mark_pos = Some((state.cursor_row, state.cursor_col));

                if let Some(line) = state.lines.get(state.cursor_row) {
                    let col = state.cursor_col.min(line.len());
                    let chars: Vec<char> = line.chars().collect();

                    // Find word start
                    let mut start = col;
                    while start > 0 && chars[start - 1].is_alphanumeric() {
                        start -= 1;
                    }
                    // Find word end
                    let mut end = col;
                    while end < chars.len() && chars[end].is_alphanumeric() {
                        end += 1;
                    }

                    state.mark_pos = Some((state.cursor_row, start));
                    state.cursor_col = end;
                }
                state.expand_region_level = 1;
            }
            1 => {
                // Expand to current line
                if let Some(line) = state.lines.get(state.cursor_row) {
                    state.mark_pos = Some((state.cursor_row, 0));
                    state.cursor_col = line.len();
                }
                state.expand_region_level = 2;
            }
            2 => {
                // Expand to current paragraph (blank-line delimited block)
                let row = state.cursor_row;
                let lines = &state.lines;

                // Find paragraph start
                let mut start_row = row;
                while start_row > 0 && !lines[start_row - 1].trim().is_empty() {
                    start_row -= 1;
                }
                // Find paragraph end
                let mut end_row = row;
                while end_row + 1 < lines.len() && !lines[end_row + 1].trim().is_empty() {
                    end_row += 1;
                }

                state.mark_pos = Some((start_row, 0));
                if let Some(last_line) = lines.get(end_row) {
                    state.cursor_row = end_row;
                    state.cursor_col = last_line.len();
                }
                state.expand_region_level = 3;
            }
            3 => {
                // Expand to entire buffer
                state.mark_pos = Some((0, 0));
                state.cursor_row = state.lines.len().saturating_sub(1);
                if let Some(last_line) = state.lines.last() {
                    state.cursor_col = last_line.len();
                }
                state.expand_region_level = 4;
            }
            _ => {
                // Already at max, do nothing
            }
        }
        Ok(Value::Nil)
    })
}

/// (contract-region) — Contract selection back one level.
fn prim_contract_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.expand_region_level == 0 {
            return Ok(Value::Nil);
        }

        state.expand_region_level -= 1;
        let level = state.expand_region_level;

        match level {
            0 => {
                // Collapse to cursor, deactivate mark
                state.mark_active = false;
                state.mark_pos = None;
            }
            1 => {
                // Back to word
                if let Some(line) = state.lines.get(state.cursor_row) {
                    let col = state.cursor_col.min(line.len());
                    let chars: Vec<char> = line.chars().collect();

                    let mut start = col;
                    while start > 0 && chars[start - 1].is_alphanumeric() {
                        start -= 1;
                    }
                    let mut end = col;
                    while end < chars.len() && chars[end].is_alphanumeric() {
                        end += 1;
                    }

                    state.mark_pos = Some((state.cursor_row, start));
                    state.cursor_col = end;
                }
            }
            2 => {
                // Back to line
                if let Some(line) = state.lines.get(state.cursor_row) {
                    state.mark_pos = Some((state.cursor_row, 0));
                    state.cursor_col = line.len();
                }
            }
            3 => {
                // Back to paragraph
                let row = state.cursor_row;
                let lines = &state.lines;
                let mut start_row = row;
                while start_row > 0 && !lines[start_row - 1].trim().is_empty() {
                    start_row -= 1;
                }
                let mut end_row = row;
                while end_row + 1 < lines.len() && !lines[end_row + 1].trim().is_empty() {
                    end_row += 1;
                }
                state.mark_pos = Some((start_row, 0));
                if let Some(last_line) = lines.get(end_row) {
                    state.cursor_row = end_row;
                    state.cursor_col = last_line.len();
                }
            }
            _ => {}
        }
        Ok(Value::Nil)
    })
}

/// (goto-last-change) — Jump to last edit position.
fn prim_goto_last_change(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some((row, col)) = state.last_changes.pop_front() {
            state.cursor_row = row.min(state.lines.len().saturating_sub(1));
            state.cursor_col = col;
        }
        Ok(Value::Nil)
    })
}

/// (hungry-delete-backward) — Delete all whitespace backward until non-whitespace.
fn prim_hungry_delete_backward(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let col = state.cursor_col;
        if let Some(line) = state.lines.get_mut(row) {
            let chars: Vec<char> = line.chars().collect();
            let col_idx = col.min(chars.len());

            // Find how many whitespace chars before cursor
            let mut delete_from = col_idx;
            while delete_from > 0 && chars[delete_from - 1].is_whitespace() {
                delete_from -= 1;
            }

            if delete_from < col_idx {
                let before: String = chars[..delete_from].iter().collect();
                let after: String = chars[col_idx..].iter().collect();
                *line = before + &after;
                state.cursor_col = delete_from;
            }
        }
        Ok(Value::Nil)
    })
}

/// (hungry-delete-forward) — Delete all whitespace forward until non-whitespace.
fn prim_hungry_delete_forward(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let col = state.cursor_col;
        if let Some(line) = state.lines.get_mut(row) {
            let chars: Vec<char> = line.chars().collect();
            let col_idx = col.min(chars.len());

            // Find how many whitespace chars after cursor
            let mut delete_to = col_idx;
            while delete_to < chars.len() && chars[delete_to].is_whitespace() {
                delete_to += 1;
            }

            if delete_to > col_idx {
                let before: String = chars[..col_idx].iter().collect();
                let after: String = chars[delete_to..].iter().collect();
                *line = before + &after;
            }
        }
        Ok(Value::Nil)
    })
}

/// (cleanup-buffer) — Delete trailing whitespace, untabify.
fn prim_cleanup_buffer(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        for line in state.lines.iter_mut() {
            // Untabify: replace tabs with spaces
            let untabbed = line.replace('\t', "    ");
            // Trim trailing whitespace
            let trimmed = untabbed.trim_end().to_string();
            *line = trimmed;
        }
        Ok(Value::Nil)
    })
}

/// (insert-empty-line) — Insert empty line after current line, move cursor there.
fn prim_insert_empty_line(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let insert_at = state.cursor_row + 1;
        state.lines.insert(insert_at, String::new());
        state.cursor_row = insert_at;
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

/// (copy-and-comment) — Copy region text and comment out original.
fn prim_copy_and_comment(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let Some((mark_row, mark_col)) = state.mark_pos else {
            return Ok(Value::Nil);
        };
        let cur_row = state.cursor_row;
        let cur_col = state.cursor_col;

        // Determine range
        let (start_row, start_col, end_row, end_col) = if mark_row < cur_row
            || (mark_row == cur_row && mark_col <= cur_col)
        {
            (mark_row, mark_col, cur_row, cur_col)
        } else {
            (cur_row, cur_col, mark_row, mark_col)
        };

        // Collect the text
        let mut region_text = String::new();
        if start_row == end_row {
            if let Some(line) = state.lines.get(start_row) {
                let chars: Vec<char> = line.chars().collect();
                let s = start_col.min(chars.len());
                let e = end_col.min(chars.len());
                region_text = chars[s..e].iter().collect();
            }
        } else {
            for row in start_row..=end_row {
                if let Some(line) = state.lines.get(row) {
                    let chars: Vec<char> = line.chars().collect();
                    if row == start_row {
                        let s = start_col.min(chars.len());
                        region_text.push_str(&chars[s..].iter().collect::<String>());
                    } else if row == end_row {
                        let e = end_col.min(chars.len());
                        region_text.push('\n');
                        region_text.push_str(&chars[..e].iter().collect::<String>());
                    } else {
                        region_text.push('\n');
                        region_text.push_str(line);
                    }
                }
            }
        }

        // Push to kill ring
        state.kill_ring.push(region_text.clone());
        state.kill_ring_idx = state.kill_ring.len() - 1;

        // Comment out each line in the range
        for row in start_row..=end_row {
            if let Some(line) = state.lines.get_mut(row) {
                if !line.starts_with("; ") {
                    *line = format!("; {}", line);
                }
            }
        }

        state.mark_active = false;
        state.mark_pos = None;
        Ok(Value::string(region_text))
    })
}
