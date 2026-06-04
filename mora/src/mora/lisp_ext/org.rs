use std::cell::RefCell;
use std::collections::HashMap;
use std::time::SystemTime;

use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

// ── Thread-local org state ───────────────────────────────────

struct ClockState {
    heading: String,
    start: SystemTime,
}

struct OrgState {
    capture_templates: HashMap<String, String>,
    clock: Option<ClockState>,
}
impl Default for OrgState {
    fn default() -> Self {
        Self {
            capture_templates: HashMap::new(),
            clock: None,
        }
    }
}

thread_local! {
    static ORG_STATE: RefCell<OrgState> = RefCell::new(OrgState::default());
}

// ── Heading helpers ──────────────────────────────────────────

fn heading_level(line: &str) -> i64 {
    let mut n = 0i64;
    for ch in line.chars() {
        if ch == '*' {
            n += 1;
        } else {
            break;
        }
    }
    n
}

fn todo_state_of(line: &str) -> &'static str {
    // After leading *, line may have a space then TODO/DELOGED? or DONE
    let trimmed = line.trim_start_matches('*').trim_start();
    if let Some(rest) = trimmed.strip_prefix("TODO") {
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
            return "TODO";
        }
    }
    if let Some(rest) = trimmed.strip_prefix("DONE") {
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
            return "DONE";
        }
    }
    "NONE"
}

fn replace_todo_state(line: &str, state: &str) -> String {
    let prefix_len = line.len() - line.trim_start_matches('*').len();
    let prefix = &line[..prefix_len];
    let rest = &line[prefix_len..];
    let after_stars = rest.trim_start();
    // Try to replace existing TODO/DONE
    for keyword in &["TODO", "DONE"] {
        if let Some(after) = after_stars.strip_prefix(keyword) {
            if after.is_empty() || after.starts_with(' ') || after.starts_with('\t') {
                let after = after.trim_start();
                return format!("{}* {} {}", prefix, state, after);
            }
        }
    }
    // No existing keyword: insert after stars
    format!("{}* {} {}", prefix, state, after_stars)
}

// ── Primitives ──────────────────────────────────────────────

/// (org-heading-level) — Return heading level at current line (number of *).
fn prim_org_heading_level(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let line = state
            .lines
            .get(state.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("");
        Ok(Value::Int(heading_level(line)))
    })
}

/// (org-todo-state) — Return TODO state at current line (TODO/DONE/NONE).
fn prim_org_todo_state(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let line = state
            .lines
            .get(state.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("");
        Ok(Value::string(todo_state_of(line)))
    })
}

/// (org-set-todo-state STATE) — Set TODO state on current line.
fn prim_org_set_todo_state(args: &[Value]) -> Result<Value, String> {
    let state_str = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        if let Some(line) = state.lines.get_mut(state.cursor_row) {
            if heading_level(line) > 0 {
                *line = replace_todo_state(line, &state_str);
                state.modified = true;
            }
        }
        Ok(Value::Nil)
    })
}

/// (org-sparse-tree PATTERN) — Return vector of matching line numbers.
fn prim_org_sparse_tree(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state(|state| {
        let matches: Vec<Value> = state
            .lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                if heading_level(line) > 0 && line.contains(&pattern) {
                    Some(Value::Int(i as i64))
                } else {
                    None
                }
            })
            .collect();
        Ok(Value::vector(matches))
    })
}

/// (org-agenda-list) — List all TODO items across buffer.
fn prim_org_agenda_list(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let items: Vec<Value> = state
            .lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                let ts = todo_state_of(line);
                if ts == "TODO" || ts == "DONE" {
                    Some(Value::vector(vec![
                        Value::Int(i as i64),
                        Value::string(line.clone()),
                    ]))
                } else {
                    None
                }
            })
            .collect();
        Ok(Value::vector(items))
    })
}

/// (org-capture-template NAME CONTENT) — Store a capture template.
fn prim_org_capture_template(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let content = extract_string(args, 1)?;
    ORG_STATE.with(|s| {
        s.borrow_mut().capture_templates.insert(name, content);
    });
    Ok(Value::Nil)
}

/// (org-capture) — Execute capture: insert template at cursor.
fn prim_org_capture(_args: &[Value]) -> Result<Value, String> {
    // Uses first template in the map (no argument = default)
    let content = ORG_STATE.with(|s| {
        let state = s.borrow();
        state
            .capture_templates
            .values()
            .next()
            .cloned()
            .unwrap_or_default()
    });
    if content.is_empty() {
        return Err("no capture templates registered".to_string());
    }
    with_editor_state_mut(|state| {
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let insert_row = state.cursor_row;
        for (i, l) in lines.into_iter().enumerate() {
            state.lines.insert(insert_row + i, l);
        }
        state.modified = true;
        Ok(Value::Nil)
    })
}

/// (org-clock-in) — Start clock on current heading.
fn prim_org_clock_in(_args: &[Value]) -> Result<Value, String> {
    let heading = with_editor_state(|state| {
        state
            .lines
            .get(state.cursor_row)
            .cloned()
            .unwrap_or_default()
    });
    ORG_STATE.with(|s| {
        s.borrow_mut().clock = Some(ClockState {
            heading,
            start: SystemTime::now(),
        });
    });
    Ok(Value::string("clock-in"))
}

/// (org-clock-out) — Stop clock and return elapsed seconds.
fn prim_org_clock_out(_args: &[Value]) -> Result<Value, String> {
    let elapsed = ORG_STATE.with(|s| {
        let mut state = s.borrow_mut();
        if let Some(clock) = state.clock.take() {
            clock
                .start
                .elapsed()
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        } else {
            -1
        }
    });
    if elapsed < 0 {
        return Err("no active clock".to_string());
    }
    Ok(Value::Int(elapsed))
}

/// (org-clock-report) — Show clocked time summary.
fn prim_org_clock_report(_args: &[Value]) -> Result<Value, String> {
    let (active, heading, elapsed) = ORG_STATE.with(|s| {
        let state = s.borrow();
        match &state.clock {
            Some(clock) => {
                let elapsed = clock
                    .start
                    .elapsed()
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                (true, clock.heading.clone(), elapsed)
            }
            None => (false, String::new(), 0i64),
        }
    });
    if active {
        Ok(Value::string(format!(
            "Clocked on: {} ({}s elapsed)",
            heading, elapsed
        )))
    } else {
        Ok(Value::string("No active clock"))
    }
}

pub fn register(ns: &mut Namespace) {
    ns.intern_with_doc(
        "org-heading-level",
        Value::Native(prim_org_heading_level),
        "Return heading level at current line.",
    );
    ns.intern_with_doc(
        "org-todo-state",
        Value::Native(prim_org_todo_state),
        "Return TODO/DONE/NONE state at current line.",
    );
    ns.intern_with_doc(
        "org-set-todo-state",
        Value::Native(prim_org_set_todo_state),
        "Set TODO state on current heading line.",
    );
    ns.intern_with_doc(
        "org-sparse-tree",
        Value::Native(prim_org_sparse_tree),
        "Return matching headline line numbers for PATTERN.",
    );
    ns.intern_with_doc(
        "org-agenda-list",
        Value::Native(prim_org_agenda_list),
        "List all TODO/DONE items as [line-number text].",
    );
    ns.intern_with_doc(
        "org-capture-template",
        Value::Native(prim_org_capture_template),
        "Store a capture template by NAME.",
    );
    ns.intern_with_doc(
        "org-capture",
        Value::Native(prim_org_capture),
        "Execute capture: insert default template at cursor.",
    );
    ns.intern_with_doc(
        "org-clock-in",
        Value::Native(prim_org_clock_in),
        "Start clock on current heading.",
    );
    ns.intern_with_doc(
        "org-clock-out",
        Value::Native(prim_org_clock_out),
        "Stop clock and return elapsed seconds.",
    );
    ns.intern_with_doc(
        "org-clock-report",
        Value::Native(prim_org_clock_report),
        "Show clocked time summary.",
    );
}
