/// Modeline and Menu Bar primitives for mora.
///
/// Emacs modeline shows: buffer name, modified flag, major mode,
/// minor modes, position, encoding, and custom segments.
///
/// Emacs menu bar: File, Edit, Options, Buffers, Tools, Help —
/// each with sub-items bound to commands.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

// ── Modeline state ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModelineSegment {
    pub name: String,
    pub content: String,
    pub face: String,
    pub order: i32,
}

#[derive(Debug)]
struct ModelineState {
    enabled: bool,
    format_string: String,
    segments: Vec<ModelineSegment>,
    /// Menu bar entries: menu_name -> [(label, action)]
    menu_bar: HashMap<String, Vec<(String, String)>>,
    menu_bar_enabled: bool,
}

thread_local! {
    static MODELINE_STATE: RefCell<ModelineState> = RefCell::new(ModelineState {
        enabled: true,
        format_string: "%m %b %M %l:%c %p".to_string(),
        segments: Vec::new(),
        menu_bar: HashMap::new(),
        menu_bar_enabled: true,
    });
}

fn with_modeline_state<R>(f: impl FnOnce(&ModelineState) -> R) -> R {
    MODELINE_STATE.with(|s| f(&s.borrow()))
}

fn with_modeline_state_mut<R>(f: impl FnOnce(&mut ModelineState) -> R) -> R {
    MODELINE_STATE.with(|s| f(&mut s.borrow_mut()))
}

// ── Format tokens ────────────────────────────────────────────

/// Build modeline string from format string and editor state.
/// Tokens:
///   %m — modified flag (** or --)
///   %b — buffer name
///   %M — major mode name
///   %m — minor modes indicators
///   %l — line number
///   %c — column number
///   %p — percentage
///   %e — encoding (UTF-8)
///   %n — narrowing indicator (Narrow)
///   %% — literal %
pub fn build_modeline_string(
    buffer_name: &str,
    major_mode: &str,
    minor_modes: &str,
    line: usize,
    col: usize,
    pct: usize,
    modified: bool,
    narrowed: bool,
    format_str: &str,
) -> String {
    let mut result = String::with_capacity(format_str.len() * 2);
    let mut chars = format_str.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.next() {
                Some('m') => result.push_str(if modified { "**" } else { "--" }),
                Some('b') => result.push_str(buffer_name),
                Some('M') => result.push_str(major_mode),
                Some('m') => result.push_str(minor_modes),
                Some('l') => result.push_str(&line.to_string()),
                Some('c') => result.push_str(&col.to_string()),
                Some('p') => result.push_str(&format!("{}%", pct)),
                Some('e') => result.push_str("UTF-8"),
                Some('n') => {
                    if narrowed {
                        result.push_str(" Narrow");
                    }
                }
                Some('%') => result.push('%'),
                Some(other) => {
                    result.push('%');
                    result.push(other);
                }
                None => result.push('%'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

// ── Menu bar ─────────────────────────────────────────────────

fn init_default_menu_bar(state: &mut ModelineState) {
    if state.menu_bar.is_empty() {
        state.menu_bar.insert("File".to_string(), vec![
            ("Open...".to_string(), "find-file".to_string()),
            ("Save".to_string(), "save-buffer".to_string()),
            ("Save As...".to_string(), "write-file".to_string()),
            ("Close".to_string(), "kill-buffer".to_string()),
            ("Quit".to_string(), "editor-quit".to_string()),
        ]);
        state.menu_bar.insert("Edit".to_string(), vec![
            ("Undo".to_string(), "undo".to_string()),
            ("Redo".to_string(), "redo".to_string()),
            ("Cut".to_string(), "kill-region".to_string()),
            ("Copy".to_string(), "copy-region".to_string()),
            ("Paste".to_string(), "yank".to_string()),
            ("Find...".to_string(), "search-forward".to_string()),
            ("Replace...".to_string(), "query-replace-pattern".to_string()),
        ]);
        state.menu_bar.insert("Buffer".to_string(), vec![
            ("Next Buffer".to_string(), "next-buffer".to_string()),
            ("Previous Buffer".to_string(), "prev-buffer".to_string()),
            ("Kill Buffer".to_string(), "kill-buffer".to_string()),
            ("List Buffers".to_string(), "buffer-list".to_string()),
        ]);
        state.menu_bar.insert("Window".to_string(), vec![
            ("Split Horizontal".to_string(), "split-window-horizontally".to_string()),
            ("Split Vertical".to_string(), "split-window-vertically".to_string()),
            ("Delete Window".to_string(), "delete-window".to_string()),
            ("Delete Other Windows".to_string(), "delete-other-windows".to_string()),
            ("Other Window".to_string(), "other-window".to_string()),
        ]);
        state.menu_bar.insert("Tools".to_string(), vec![
            ("Shell Command".to_string(), "shell-command".to_string()),
            ("Grep".to_string(), "grep".to_string()),
            ("Project Find".to_string(), "project-find-file".to_string()),
        ]);
        state.menu_bar.insert("Help".to_string(), vec![
            ("Describe Function".to_string(), "describe-function".to_string()),
            ("Version".to_string(), "editor-status".to_string()),
        ]);
    }
}

// ── Lisp primitives ──────────────────────────────────────────

/// (modeline-toggle) → toggle modeline on/off
fn prim_modeline_toggle(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state_mut(|state| {
        state.enabled = !state.enabled;
        Ok(Value::Bool(state.enabled))
    })
}

/// (modeline-enabled?) → is modeline enabled?
fn prim_modeline_enabled(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state(|state| Ok(Value::Bool(state.enabled)))
}

/// (modeline-set-format FORMAT) → set modeline format string
fn prim_modeline_set_format(args: &[Value]) -> Result<Value, String> {
    let format = extract_string(args, 0)?;
    with_modeline_state_mut(|state| {
        state.format_string = format;
        Ok(Value::Nil)
    })
}

/// (modeline-get-format) → get current format string
fn prim_modeline_get_format(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state(|state| Ok(Value::string(&state.format_string)))
}

/// (modeline-add-segment NAME CONTENT FACE ORDER) → add a custom segment
fn prim_modeline_add_segment(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let content = extract_string(args, 1)?;
    let face = args.get(2)
        .and_then(|v| match v { Value::String(s) => Some(s.to_string()), _ => None })
        .unwrap_or_else(|| "default".to_string());
    let order = args.get(3)
        .and_then(|v| match v { Value::Int(n) => Some(*n as i32), _ => None })
        .unwrap_or(0);

    with_modeline_state_mut(|state| {
        // Replace existing segment with same name, or add new
        if let Some(seg) = state.segments.iter_mut().find(|s| s.name == name) {
            seg.content = content;
            seg.face = face;
            seg.order = order;
        } else {
            state.segments.push(ModelineSegment { name, content, face, order });
        }
        state.segments.sort_by_key(|s| s.order);
        Ok(Value::Nil)
    })
}

/// (modeline-remove-segment NAME) → remove a custom segment
fn prim_modeline_remove_segment(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    with_modeline_state_mut(|state| {
        state.segments.retain(|s| s.name != name);
        Ok(Value::Nil)
    })
}

/// (modeline-segments) → vector of segment names
fn prim_modeline_segments(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state(|state| {
        Ok(Value::vector(
            state.segments.iter().map(|s| Value::string(&s.name)).collect(),
        ))
    })
}

/// (modeline-render) → render modeline string using current format + editor state
fn prim_modeline_render(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state(|modeline_state| {
        with_editor_state(|state| {
            let buffer_name = state.file_path.as_deref().unwrap_or("*scratch*");
            let major_mode = state.mode.as_str();
            let minor_modes_str = ""; // minor mode indicators from editor

            let total_lines = state.lines.len().max(1);
            let pct = if total_lines <= 1 { 100 } else { ((state.cursor_row + 1) * 100) / total_lines };

            let mut rendered = build_modeline_string(
                buffer_name,
                major_mode,
                minor_modes_str,
                state.cursor_row + 1,
                state.cursor_col + 1,
                pct,
                state.modified,
                state.narrow_start.is_some(),
                &modeline_state.format_string,
            );

            // Append custom segments
            for seg in &modeline_state.segments {
                rendered.push(' ');
                rendered.push_str(&seg.content);
            }

            Ok(Value::string(rendered))
        })
    })
}

/// (menu-bar-list) → vector of menu names
fn prim_menu_bar_list(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state_mut(|state| {
        if state.menu_bar.is_empty() {
            init_default_menu_bar(state);
        }
        let mut names: Vec<String> = state.menu_bar.keys().cloned().collect();
        names.sort();
        Ok(Value::vector(
            names.into_iter().map(Value::string).collect(),
        ))
    })
}

/// (menu-bar-items MENU) → vector of [label action] for given menu
fn prim_menu_bar_items(args: &[Value]) -> Result<Value, String> {
    let menu = extract_string(args, 0)?;
    with_modeline_state_mut(|state| {
        if state.menu_bar.is_empty() {
            init_default_menu_bar(state);
        }
        match state.menu_bar.get(&menu) {
            Some(items) => {
                let pairs: Vec<Value> = items.iter()
                    .map(|(label, action)| Value::vector(vec![
                        Value::string(label),
                        Value::string(action),
                    ]))
                    .collect();
                Ok(Value::vector(pairs))
            }
            None => Ok(Value::vector(vec![])),
        }
    })
}

/// (menu-bar-add MENU LABEL ACTION) → add item to menu
fn prim_menu_bar_add(args: &[Value]) -> Result<Value, String> {
    let menu = extract_string(args, 0)?;
    let label = extract_string(args, 1)?;
    let action = extract_string(args, 2)?;
    with_modeline_state_mut(|state| {
        if state.menu_bar.is_empty() {
            init_default_menu_bar(state);
        }
        state.menu_bar.entry(menu).or_default().push((label, action));
        Ok(Value::Nil)
    })
}

/// (menu-bar-remove MENU LABEL) → remove item from menu
fn prim_menu_bar_remove(args: &[Value]) -> Result<Value, String> {
    let menu = extract_string(args, 0)?;
    let label = extract_string(args, 1)?;
    with_modeline_state_mut(|state| {
        if state.menu_bar.is_empty() {
            init_default_menu_bar(state);
        }
        if let Some(items) = state.menu_bar.get_mut(&menu) {
            items.retain(|(l, _)| l != &label);
        }
        Ok(Value::Nil)
    })
}

/// (menu-bar-toggle) → toggle menu bar on/off
fn prim_menu_bar_toggle(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state_mut(|state| {
        state.menu_bar_enabled = !state.menu_bar_enabled;
        Ok(Value::Bool(state.menu_bar_enabled))
    })
}

/// (menu-bar-enabled?) → is menu bar enabled?
fn prim_menu_bar_enabled(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state(|state| Ok(Value::Bool(state.menu_bar_enabled)))
}

/// (menu-bar-render) → render menu bar string (first line)
fn prim_menu_bar_render(_args: &[Value]) -> Result<Value, String> {
    with_modeline_state_mut(|state| {
        if !state.menu_bar_enabled {
            return Ok(Value::string(""));
        }
        if state.menu_bar.is_empty() {
            init_default_menu_bar(state);
        }
        let mut names: Vec<&String> = state.menu_bar.keys().collect();
        names.sort();
        let bar = names.iter()
            .map(|n| format!(" {} ", n))
            .collect::<Vec<_>>()
            .join("|");
        Ok(Value::string(bar))
    })
}
// ── Registration ─────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("modeline-toggle", Value::Native(prim_modeline_toggle),
        "Toggle modeline visibility on/off.");
    ns.intern_with_doc("modeline-enabled?", Value::Native(prim_modeline_enabled),
        "Return t if modeline is enabled.");
    ns.intern_with_doc("modeline-set-format", Value::Native(prim_modeline_set_format),
        "Set modeline format string. Tokens: %m %b %M %l %c %p %e %n %%.");
    ns.intern_with_doc("modeline-get-format", Value::Native(prim_modeline_get_format),
        "Get current modeline format string.");
    ns.intern_with_doc("modeline-add-segment", Value::Native(prim_modeline_add_segment),
        "Add custom segment to modeline: (modeline-add-segment NAME CONTENT FACE ORDER).");
    ns.intern_with_doc("modeline-remove-segment", Value::Native(prim_modeline_remove_segment),
        "Remove custom modeline segment by NAME.");
    ns.intern_with_doc("modeline-segments", Value::Native(prim_modeline_segments),
        "Return vector of custom modeline segment names.");
    ns.intern_with_doc("modeline-render", Value::Native(prim_modeline_render),
        "Render the modeline string for the current buffer.");
    ns.intern_with_doc("menu-bar-list", Value::Native(prim_menu_bar_list),
        "Return vector of menu names in the menu bar.");
    ns.intern_with_doc("menu-bar-items", Value::Native(prim_menu_bar_items),
        "Return vector of [label action] pairs for MENU.");
    ns.intern_with_doc("menu-bar-add", Value::Native(prim_menu_bar_add),
        "Add an item to MENU with LABEL bound to ACTION.");
    ns.intern_with_doc("menu-bar-remove", Value::Native(prim_menu_bar_remove),
        "Remove item with LABEL from MENU.");
    ns.intern_with_doc("menu-bar-toggle", Value::Native(prim_menu_bar_toggle),
        "Toggle menu bar visibility on/off.");
    ns.intern_with_doc("menu-bar-enabled?", Value::Native(prim_menu_bar_enabled),
        "Return t if menu bar is enabled.");
    ns.intern_with_doc("menu-bar-render", Value::Native(prim_menu_bar_render),
        "Render the menu bar as a single string line.");
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::editor_state::*;

    fn setup() {
        set_editor_state(EditorState::new());
    }

    fn teardown() {
        take_editor_state();
    }

    #[test]
    fn test_modeline_render_default() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let rendered = bridge.eval("(modeline-render)").unwrap();
        match rendered {
            Value::String(s) => {
                assert!(s.contains("--"), "should show unmodified flag: {}", s);
                assert!(s.contains("*scratch*"), "should show buffer name: {}", s);
                assert!(s.contains("1:"), "should show cursor pos: {}", s);
            }
            _ => panic!("expected string"),
        }
        teardown();
    }

    #[test]
    fn test_modeline_render_modified() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"hello\")").unwrap();
        let rendered = bridge.eval("(modeline-render)").unwrap();
        match rendered {
            Value::String(s) => {
                assert!(s.contains("**"), "should show modified flag: {}", s);
            }
            _ => panic!("expected string"),
        }
        teardown();
    }

    #[test]
    fn test_modeline_format_tokens() {
        let result = build_modeline_string(
            "main.rs", "rust", "AI", 42, 10, 75, true, false, "[%m] %b (%M) %l:%c %p"
        );
        assert_eq!(result, "[**] main.rs (rust) 42:10 75%");
    }

    #[test]
    fn test_modeline_toggle() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        assert_eq!(bridge.eval("(modeline-enabled?)").unwrap(), Value::Bool(true));
        bridge.eval("(modeline-toggle)").unwrap();
        assert_eq!(bridge.eval("(modeline-enabled?)").unwrap(), Value::Bool(false));
        bridge.eval("(modeline-toggle)").unwrap();
        assert_eq!(bridge.eval("(modeline-enabled?)").unwrap(), Value::Bool(true));
        teardown();
    }

    #[test]
    fn test_modeline_custom_format() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval("(modeline-set-format \"[%b] %l:%c\")").unwrap();
        assert_eq!(
            bridge.eval("(modeline-get-format)").unwrap(),
            Value::string("[%b] %l:%c")
        );

        let rendered = bridge.eval("(modeline-render)").unwrap();
        match rendered {
            Value::String(s) => {
                assert!(s.contains("[*scratch*]"), "should use custom format: {}", s);
            }
            _ => panic!("expected string"),
        }
        teardown();
    }

    #[test]
    fn test_modeline_custom_segments() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval("(modeline-add-segment \"vcs\" \"[main]\" \"bold\" 10)").unwrap();
        let segments = bridge.eval("(modeline-segments)").unwrap();
        match segments {
            Value::Vector(v) => assert_eq!(v.len(), 1),
            _ => panic!("expected vector"),
        }

        let rendered = bridge.eval("(modeline-render)").unwrap();
        match rendered {
            Value::String(s) => assert!(s.contains("[main]"), "should show custom segment: {}", s),
            _ => panic!("expected string"),
        }

        bridge.eval("(modeline-remove-segment \"vcs\")").unwrap();
        let segments = bridge.eval("(modeline-segments)").unwrap();
        match segments {
            Value::Vector(v) => assert_eq!(v.len(), 0),
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_menu_bar_list() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let menus = bridge.eval("(menu-bar-list)").unwrap();
        match menus {
            Value::Vector(v) => assert!(v.len() >= 6, "should have default menus"),
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_menu_bar_items() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let items = bridge.eval("(menu-bar-items \"File\")").unwrap();
        match items {
            Value::Vector(v) => {
                assert!(v.len() >= 5, "File menu should have items");
                // First item should be [label, action]
                match &v[0] {
                    Value::Vector(pair) => {
                        assert_eq!(pair.len(), 2);
                    }
                    _ => panic!("expected pair vector"),
                }
            }
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_menu_bar_add_and_remove() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval(r#"(menu-bar-add "Tools" "Compile" "compile-command")"#).unwrap();
        let items = bridge.eval("(menu-bar-items \"Tools\")").unwrap();
        match items {
            Value::Vector(v) => {
                assert!(v.len() >= 4, "Tools menu should have items");
            }
            _ => panic!("expected vector"),
        }

        bridge.eval(r#"(menu-bar-remove "Tools" "Compile")"#).unwrap();
        teardown();
    }

    #[test]
    fn test_menu_bar_render() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let bar = bridge.eval("(menu-bar-render)").unwrap();
        match bar {
            Value::String(s) => {
                assert!(s.contains("File"), "should have File menu: {}", s);
                assert!(s.contains("Edit"), "should have Edit menu: {}", s);
            }
            _ => panic!("expected string"),
        }

        bridge.eval("(menu-bar-toggle)").unwrap();
        assert_eq!(bridge.eval("(menu-bar-enabled?)").unwrap(), Value::Bool(false));

        let bar = bridge.eval("(menu-bar-render)").unwrap();
        match bar {
            Value::String(s) => assert!(s.is_empty(), "should be empty when disabled"),
            _ => panic!("expected string"),
        }
        teardown();
    }

    #[test]
    fn test_narrowing_indicator() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval(r#"(buffer-set-content "line0\nline1\nline2")"#).unwrap();
        bridge.eval(r#"(narrow-to-region 1 2)"#).unwrap();
        bridge.eval(r#"(modeline-set-format "%m %b %M %l:%c %p%n")"#).unwrap();
        let rendered = bridge.eval(r#"(modeline-render)"#).unwrap();
        match rendered {
            Value::String(s) => assert!(s.contains("Narrow"), "should show narrowing: {}", s),
            _ => panic!("expected string"),
        }
        teardown();
    }
}
