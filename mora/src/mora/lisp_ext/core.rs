use std::collections::HashMap;

use crate::lisp::eval::{EvalError, Evaluator};
use crate::lisp::types::Value;

use super::editor_state::*;
use super::command;

// Re-export from editor_state and submodules for backward compat
pub use super::editor_state::{
    EditorState, set_editor_state, take_editor_state, with_editor_state, with_editor_state_mut,
};

pub struct MoraLispBridge {
    pub evaluator: Evaluator,
}

impl MoraLispBridge {
    pub fn new() -> Self {
        command::clear_command_registry();
        let mut evaluator = Evaluator::new();
        Self::register_editor_primitives(&mut evaluator);
        Self { evaluator }
    }

    fn register_editor_primitives(eval: &mut Evaluator) {
        let buffer_ns = eval.ns.find_or_create("mora.buffer");
        let cursor_ns = eval.ns.find_or_create("mora.cursor");
        let mode_ns = eval.ns.find_or_create("mora.mode");
        let editor_ns = eval.ns.find_or_create("mora.editor");
        let window_ns = eval.ns.find_or_create("mora.window");
        let hook_ns = eval.ns.find_or_create("mora.hook");
        let keymap_ns = eval.ns.find_or_create("mora.keymap");
        let overlay_ns = eval.ns.find_or_create("mora.overlay");
        let shell_ns = eval.ns.find_or_create("mora.shell");
        let ui_ns = eval.ns.find_or_create("mora.ui");
        let command_ns = eval.ns.find_or_create("mora.command");
        let kill_ring_ns = eval.ns.find_or_create("mora.kill-ring");
        let mark_ns = eval.ns.find_or_create("mora.mark");
        let register_ns = eval.ns.find_or_create("mora.register");
        let var_ns = eval.ns.find_or_create("mora.var");
        let minibuffer_ns = eval.ns.find_or_create("mora.minibuffer");
        let region_ns = eval.ns.find_or_create("mora.region");
        let undo_ns = eval.ns.find_or_create("mora.undo");
        let editing_ns = eval.ns.find_or_create("mora.editing");
        let tramp_ns = eval.ns.find_or_create("mora.tramp");
        let history_ns = eval.ns.find_or_create("mora.history");
        let visual_ns = eval.ns.find_or_create("mora.visual");

        // Editor operations + constants (these stay in core since they're simple)
        let mut ns = editor_ns.lock();
        ns.intern("editor-message", Value::Native(prim_editor_message));
        ns.intern_private("message", Value::Native(prim_editor_message));
        ns.intern("editor-quit", Value::Native(prim_editor_quit));
        ns.intern_private("quit", Value::Native(prim_editor_quit));
        ns.intern("editor-status", Value::Native(prim_editor_status));
        ns.intern_private("status", Value::Native(prim_editor_status));
        ns.intern("*mora-version*", Value::string(env!("CARGO_PKG_VERSION")));
        ns.intern("*newline*", Value::string("\n"));
        ns.intern("*tab*", Value::string("\t"));
        drop(ns);

        // UI DSL primitives (simple ones stay here, converter is public below)
        let mut ns = ui_ns.lock();
        ns.intern("register-ui-builder", Value::Native(prim_register_ui_builder));
        ns.intern_private("register-builder", Value::Native(prim_register_ui_builder));
        ns.intern("ui-builders", Value::Native(prim_ui_builders));
        ns.intern_private("builders", Value::Native(prim_ui_builders));
        drop(ns);

        // Delegate to submodule register() functions
        super::buffer::register(&mut buffer_ns.lock());
        super::cursor::register(&mut cursor_ns.lock());
        super::mode::register(&mut mode_ns.lock());
        super::window::register(&mut window_ns.lock());
        super::hook::register(&mut hook_ns.lock());
        super::overlay::register(&mut overlay_ns.lock());
        super::shell::register(&mut shell_ns.lock());
        super::command::register(&mut command_ns.lock());
        super::kill_ring::register(&mut kill_ring_ns.lock());
        super::mark::register(&mut mark_ns.lock());
        super::register::register(&mut register_ns.lock());
        super::var::register(&mut var_ns.lock());
        super::minibuffer::register(&mut minibuffer_ns.lock());
        super::region::register(&mut region_ns.lock());
        super::undo::register(&mut undo_ns.lock());
        super::editing::register(&mut editing_ns.lock());
        super::tramp::register(&mut tramp_ns.lock());
        super::history::register(&mut history_ns.lock());
        super::visual::register(&mut visual_ns.lock());
        // Note: search primitives are registered by buffer::register()

        // Also register keymap operations (define-key goes in hook namespace)
        // This is handled by hook::register() since it adds to state.keybindings

        for ns_name in [
            "mora.buffer", "mora.cursor", "mora.mode", "mora.editor",
            "mora.window", "mora.hook", "mora.keymap", "mora.overlay",
            "mora.shell", "mora.ui", "mora.command", "mora.kill-ring",
            "mora.editing",
            "mora.mark", "mora.register", "mora.var", "mora.minibuffer",
            "mora.region", "mora.undo", "mora.tramp", "mora.history",
            "mora.visual",
        ] {
            eval.ns
                .refer_all(ns_name, "user")
                .expect("Mora host namespaces and user namespace exist");
        }
    }
    pub fn eval(&mut self, code: &str) -> Result<Value, EvalError> {
        let forms = self.evaluator.read_cached(code)?;
        Ok(crate::lisp::vm::compile_and_run(&mut self.evaluator, &forms)?)
    }
    pub fn load_init_file(&mut self) {
        let home_init = dirs::home_dir().map(|h| h.join(".mora").join("init.mora"));
        let local_init = std::path::PathBuf::from(".mora/init.mora");
        for path in [home_init, Some(local_init)].into_iter().flatten() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(code) => {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            self.eval(&code)
                        }));
                        match result {
                            Ok(Ok(_)) => {}
                            Ok(Err(e)) => {
                                eprintln!("Error loading {}: {}", path.display(), e);
                            }
                            Err(_) => {
                                eprintln!("Panic loading {}: init file caused a crash", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading {}: {}", path.display(), e);
                    }
                }
                break;
            }
        }
    }
    pub fn command_names(&self) -> Vec<String> {
        command::command_names()
    }

    pub fn has_command(&self, name: &str) -> bool {
        command::resolve_command_name(name).is_some()
    }

    pub fn execute_command(&mut self, name: &str) -> Result<Option<Value>, EvalError> {
        let Some(name) = command::resolve_command_name(name) else {
            return Ok(None);
        };
        let entry = command::command_entry(&name)
            .ok_or_else(|| EvalError::Custom(format!("command not found: {}", name)))?;
        let value = match entry.func {
            Value::Fn(f) => self.evaluator.call_fn(f, vec![])?,
            Value::Native(f) => f(&[]).map_err(EvalError::Custom)?,
            other => {
                return Err(EvalError::NotAFunction(format!(
                    "command {} is {}",
                    name,
                    other.type_name()
                )))
            }
        };
        Ok(Some(value))
    }
}

impl Default for MoraLispBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ── Editor primitives (simple, stay in core) ─────────────────

fn prim_editor_message(args: &[Value]) -> Result<Value, String> {
    let msg = match args.first() {
        Some(Value::String(s)) => s.to_string(),
        Some(v) => format!("{}", v),
        None => String::new(),
    };
    with_editor_state_mut(|state| {
        state.status_message = msg.clone();
        Ok(Value::string(msg))
    })
}

fn prim_editor_quit(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.quit_requested = true;
        Ok(Value::Nil)
    })
}

fn prim_editor_status(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        Ok(Value::string(format!(
            "Mora | {} | {}{}",
            state.file_path.as_deref().unwrap_or("*scratch*"),
            state.mode,
            if state.modified { " [+]" } else { "" }
        )))
    })
}

fn prim_register_ui_builder(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("register-ui-builder requires 1 argument (a function)".to_string());
    }
    let builder = args[0].clone();
    with_editor_state_mut(|state| {
        state.ui_builders.push(builder);
        Ok(Value::Nil)
    })
}

fn prim_ui_builders(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.ui_builders.len() as i64)))
}

// ── Lisp Value → UiNode converter ────────────────────────────

fn map_get_kw<'a>(
    map: &'a std::collections::HashMap<Value, Value>,
    key: &str,
) -> Option<&'a Value> {
    map.get(&Value::keyword(key))
}

fn map_get_str_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<String> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        Value::Keyword(k) => Some(k.name.to_string()),
        _ => None,
    })
}

fn map_get_bool_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<bool> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn map_get_u16_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<u16> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Int(i) => Some(*i as u16),
        _ => None,
    })
}

fn map_get_f64_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<f64> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    })
}

fn parse_color_str(s: &str) -> display_protocol::Color {
    match parse_color(s) {
        Some(c) => display_protocol::Color::new(c.r, c.g, c.b),
        None => display_protocol::Color::WHITE,
    }
}

fn lisp_value_to_style(val: &Value) -> display_protocol::Style {
    let mut style = display_protocol::Style::default();
    if let Value::Map(map) = val {
        if let Some(fg) = map_get_str_kw(map, "fg") {
            style.fg = Some(parse_color_str(&fg));
        }
        if let Some(color) = map_get_str_kw(map, "color") {
            style.fg = Some(parse_color_str(&color));
        }
        if let Some(bg) = map_get_str_kw(map, "bg") {
            style.bg = Some(parse_color_str(&bg));
        }
        if let Some(b) = map_get_bool_kw(map, "bold") { style.bold = b; }
        if let Some(b) = map_get_bool_kw(map, "italic") { style.italic = b; }
        if let Some(b) = map_get_bool_kw(map, "underline") { style.underline = b; }
        if let Some(b) = map_get_bool_kw(map, "dim") { style.dim = b; }
        if let Some(b) = map_get_bool_kw(map, "strikethrough") { style.strikethrough = b; }
        if let Some(b) = map_get_bool_kw(map, "reverse") { style.reverse = b; }
        if let Some(b) = map_get_bool_kw(map, "blink") { style.blink = b; }
    }
    style
}

fn apply_style_to_text(
    mut node: display_protocol::UiNode,
    style: &display_protocol::Style,
) -> display_protocol::UiNode {
    if *style != display_protocol::Style::default() {
        node = node.bold().dim();
        if style.bold { node = node.bold(); }
        if style.dim { node = node.dim(); }
        if style.italic { node = node.italic(); }
        if style.underline { node = node.underline(); }
        if let Some(fg) = style.fg { node = node.color(fg); }
        if let Some(bg) = style.bg { node = node.bg(bg); }
    }
    node
}

pub fn lisp_value_to_uinode(val: &Value) -> display_protocol::UiNode {
    match val {
        Value::Nil => display_protocol::UiNode::None,
        Value::Bool(b) => display_protocol::UiNode::text(b.to_string()),
        Value::Int(i) => display_protocol::UiNode::text(i.to_string()),
        Value::Float(f) => display_protocol::UiNode::text(f.to_string()),
        Value::String(s) => display_protocol::UiNode::text(s.to_string()),
        Value::Vector(v) => {
            let children: Vec<display_protocol::UiNode> =
                v.iter().map(lisp_value_to_uinode).collect();
            display_protocol::UiNode::column(children)
        }
        Value::Map(map) => {
            let node_type = map_get_str_kw(map, "type").unwrap_or_default();
            let style = map_get_kw(map, "style")
                .map(lisp_value_to_style)
                .unwrap_or_default();

            match node_type.as_str() {
                "text" => {
                    let content = map_get_str_kw(map, "content").unwrap_or_default();
                    let mut node = display_protocol::UiNode::text(content);
                    node = apply_style_to_text(node, &style);
                    node
                }
                "span" => {
                    let content = map_get_str_kw(map, "content").unwrap_or_default();
                    display_protocol::UiNode::span(content, style)
                }
                "column" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::column(children);
                    node = apply_flex_props(node, map);
                    node
                }
                "row" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::row(children);
                    node = apply_flex_props(node, map);
                    node
                }
                "box" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::boxed(children);
                    node = apply_box_props(node, map);
                    node
                }
                "divider" => {
                    let node = display_protocol::UiNode::divider();
                    if let Some(ch) = map_get_str_kw(map, "char").and_then(|s| s.chars().next()) {
                        match node {
                            display_protocol::UiNode::Divider(d) => {
                                display_protocol::UiNode::Divider(d.char(ch))
                            }
                            other => other,
                        }
                    } else {
                        node
                    }
                }
                "progress" => {
                    let value = map_get_f64_kw(map, "value").unwrap_or(0.0) as f32;
                    let max = map_get_f64_kw(map, "max").unwrap_or(100.0) as f32;
                    display_protocol::UiNode::progress(value, max)
                }
                "list" => {
                    let children = extract_children(map);
                    display_protocol::UiNode::list(children)
                }
                "show" => {
                    let when = map_get_bool_kw(map, "when").unwrap_or(false);
                    let child = map_get_kw(map, "child")
                        .map(lisp_value_to_uinode)
                        .unwrap_or(display_protocol::UiNode::None);
                    display_protocol::UiNode::show(when, child)
                }
                "for" => {
                    let children = if let Some(coll) = map_get_kw(map, "coll") {
                        let func = map_get_kw(map, "func");
                        match (func, coll) {
                            (Some(_f), Value::Vector(v)) | (Some(_f), Value::List(v)) => {
                                v.iter().map(lisp_value_to_uinode).collect()
                            }
                            _ => extract_children(map),
                        }
                    } else {
                        extract_children(map)
                    };
                    display_protocol::UiNode::For { children }
                }
                "scroll-view" => {
                    let child = map_get_kw(map, "child")
                        .map(lisp_value_to_uinode)
                        .unwrap_or(display_protocol::UiNode::None);
                    let mut scroll_top = 0u16;
                    let mut height = 10u16;
                    if let Some(props) = map_get_kw(map, "props").and_then(|v| match v {
                        Value::Map(m) => Some(m.as_ref()),
                        _ => None,
                    }) {
                        scroll_top = map_get_u16_kw(props, "scroll-top").unwrap_or(0);
                        height = map_get_u16_kw(props, "height").unwrap_or(10);
                    }
                    display_protocol::UiNode::ScrollView(display_protocol::ScrollNode {
                        child: Box::new(child),
                        scroll_y: scroll_top as u32,
                        scroll_x: 0,
                        viewport_width: 80,
                        viewport_height: height,
                        content_height: None,
                        content_width: None,
                        virtual_scroll: false,
                        scroll_policy: display_protocol::ScrollPolicy::Auto,
                    })
                }
                "button" => {
                    let label = map_get_str_kw(map, "label").unwrap_or_default();
                    display_protocol::UiNode::text(label)
                }
                _ => display_protocol::UiNode::None,
            }
        }
        Value::List(v) => {
            let children: Vec<display_protocol::UiNode> =
                v.iter().map(lisp_value_to_uinode).collect();
            display_protocol::UiNode::column(children)
        }
        _ => display_protocol::UiNode::text(format!("{}", val)),
    }
}

fn extract_children(
    map: &std::collections::HashMap<Value, Value>,
) -> Vec<display_protocol::UiNode> {
    match map_get_kw(map, "children") {
        Some(Value::Vector(v)) => v.iter().map(lisp_value_to_uinode).collect(),
        Some(Value::List(v)) => v.iter().map(lisp_value_to_uinode).collect(),
        _ => Vec::new(),
    }
}
fn apply_flex_props(
    mut node: display_protocol::UiNode,
    map: &std::collections::HashMap<Value, Value>,
) -> display_protocol::UiNode {
    if let Some(gap) = map_get_f64_kw(map, "gap") {
        node = node.gap(gap as u16);
    }
    if let Some(align) = map_get_str_kw(map, "align") {
        let a = match align.as_str() {
            "start" => display_protocol::Align::Start,
            "center" => display_protocol::Align::Center,
            "end" => display_protocol::Align::End,
            "stretch" => display_protocol::Align::Stretch,
            _ => display_protocol::Align::Start,
        };
        node = node.align(a);
    }
    if let Some(justify) = map_get_str_kw(map, "justify") {
        let j = match justify.as_str() {
            "start" => display_protocol::Justify::Start,
            "center" => display_protocol::Justify::Center,
            "end" => display_protocol::Justify::End,
            "space-between" => display_protocol::Justify::SpaceBetween,
            "space-around" => display_protocol::Justify::SpaceAround,
            _ => display_protocol::Justify::Start,
        };
        node = node.justify(j);
    }
    node
}
fn apply_box_props(
    mut node: display_protocol::UiNode,
    map: &std::collections::HashMap<Value, Value>,
) -> display_protocol::UiNode {
    if let Some(title) = map_get_str_kw(map, "title") {
        node = node.title(title);
    }
    if map_get_bool_kw(map, "border").unwrap_or(false) {
        node = node.border(display_protocol::Border {
            top: true, right: true, bottom: true, left: true, style: None,
        });
    }
    if let Some(padding) = map_get_f64_kw(map, "padding") {
        let p = padding as u16;
        node = node.padding(display_protocol::Padding {
            top: p, right: p, bottom: p, left: p,
        });
    }
    if let Some(style) = map_get_kw(map, "style") {
        let s = lisp_value_to_style(style);
        if let Some(fg) = s.fg { node = node.color(fg); }
        if let Some(bg) = s.bg { node = node.bg(bg); }
    }
    node
}
pub fn build_lisp_ui(width: u16, height: u16) -> Option<display_protocol::UiNode> {
    let mut nodes = Vec::new();
    let builder_count = with_editor_state(|state| state.ui_builders.len());
    for i in 0..builder_count {
        let builder = with_editor_state(|state| state.ui_builders.get(i).cloned());
        if let Some(func) = builder {
            let result = crate::lisp::eval::with_evaluator(|eval| {
                match func {
                    Value::Fn(f) => eval.call_fn(f, vec![Value::Int(width as i64), Value::Int(height as i64)]),
                    Value::Native(f) => f(&[Value::Int(width as i64), Value::Int(height as i64)]).map_err(|e| crate::lisp::eval::EvalError::Custom(e)),
                    _ => Err(crate::lisp::eval::EvalError::NotAFunction("ui builder is not callable".to_string())),
                }
            });
            match result {
                Ok(val) => {
                    nodes.push(lisp_value_to_uinode(&val));
                }
                Err(_) => {}
            }
        }
    }
    if nodes.is_empty() {
        None
    } else if nodes.len() == 1 {
        Some(nodes.into_iter().next().unwrap())
    } else {
        Some(display_protocol::UiNode::column(nodes))
    }
}

fn parse_color(s: &str) -> Option<super::super::display::style::MoraColor> {
    let s = s.trim();
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        Some(super::super::display::style::MoraColor::new(r, g, b))
    } else {
        match s.to_lowercase().as_str() {
            "red" => Some(super::super::display::style::MoraColor::new(255, 0, 0)),
            "green" => Some(super::super::display::style::MoraColor::new(0, 255, 0)),
            "blue" => Some(super::super::display::style::MoraColor::new(0, 0, 255)),
            "yellow" => Some(super::super::display::style::MoraColor::new(255, 255, 0)),
            "cyan" => Some(super::super::display::style::MoraColor::new(0, 255, 255)),
            "magenta" => Some(super::super::display::style::MoraColor::new(255, 0, 255)),
            "white" => Some(super::super::display::style::MoraColor::new(255, 255, 255)),
            "black" => Some(super::super::display::style::MoraColor::new(0, 0, 0)),
            "orange" => Some(super::super::display::style::MoraColor::new(255, 165, 0)),
            "gray" | "grey" => Some(super::super::display::style::MoraColor::new(128, 128, 128)),
            _ => None,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_primitives_remain_available_unqualified() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(editor-message \"legacy\")").unwrap();

        with_editor_state(|state| {
            assert_eq!(state.status_message, "legacy");
        });
        take_editor_state();
    }

    #[test]
    fn defcommand_registers_and_executes_editor_command() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defcommand say-hello
                  "Say hello from a user command."
                  []
                  (editor/message "hello from command"))
                "#,
            )
            .unwrap();

        assert!(bridge.has_command("say-hello"));
        bridge.execute_command("say-hello").unwrap();
        with_editor_state(|state| {
            assert_eq!(state.status_message, "hello from command");
        });
        take_editor_state();
    }

    #[test]
    fn interactive_defn_registers_and_executes_editor_command() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defn say-hello
                  "Say hello through interactive defn."
                  []
                  (interactive)
                  (editor/message "hello from interactive defn"))
                "#,
            )
            .unwrap();

        assert!(bridge.has_command("say-hello"));
        bridge.execute_command("say-hello").unwrap();
        with_editor_state(|state| {
            assert_eq!(state.status_message, "hello from interactive defn");
        });
        take_editor_state();
    }
}
