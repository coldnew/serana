use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ns::Namespace;
use crate::types::Value;
pub mod string;
pub mod core;
pub mod math;


macro_rules! builtin_fn {
    ($name:expr, $func:expr) => {{
        let sym = Symbol {
            ns: Some(Arc::new("mora.core".to_string())),
            name: Arc::new($name.to_string()),
        };
        Var::new(
            sym,
            Value::Fn(crate::types::FnValue {
                name: Some($name.to_string()),
                params: vec![],
                body: Arc::new(vec![]),
                closure: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
                is_macro: false,
                meta: None,
            }),
        )
    }};
}

pub fn register_builtins(ns: &mut Namespace) {
    core::register(ns);
    string::register(ns);
    math::register(ns);

    // --- GC ---
    register_native(ns, "gc-collect", native_gc_collect);
    register_native(ns, "gc-count", native_gc_count);
    register_native(ns, "gc-alloc", native_gc_alloc);
    register_native(ns, "gc-deref", native_gc_deref);

    // --- UI DSL (Reagent-like) ---
    register_native(ns, "text", native_ui_text);
    register_native(ns, "span", native_ui_span);
    register_native(ns, "column", native_ui_column);
    register_native(ns, "row", native_ui_row);
    register_native(ns, "box", native_ui_box);
    register_native(ns, "divider", native_ui_divider);
    register_native(ns, "progress", native_ui_progress);
    register_native(ns, "ui-list", native_ui_list);
    register_native(ns, "show", native_ui_show);
    register_native(ns, "for-each", native_ui_for_each);
    register_native(ns, "scroll-view", native_ui_scroll_view);
    register_native(ns, "button", native_ui_button);
}

pub(crate) fn register_native(ns: &mut Namespace, name: &str, func: fn(&[Value]) -> Result<Value, String>) {
    ns.intern(name, Value::Native(func));
}

use parking_lot::Mutex;
use std::sync::OnceLock;

type NativeFn = fn(&[Value]) -> Result<Value, String>;

static NATIVE_REGISTRY: OnceLock<Mutex<HashMap<String, NativeFn>>> = OnceLock::new();

fn get_registry() -> &'static Mutex<HashMap<String, NativeFn>> {
    NATIVE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_native_global(name: &str, func: NativeFn) {
    get_registry().lock().insert(name.to_string(), func);
}

pub fn call_native(name: &str, args: &[Value]) -> Result<Value, String> {
    let registry = get_registry().lock();
    registry
        .get(name)
        .map(|f| f(args))
        .unwrap_or_else(|| Err(format!("unknown native function: {}", name)))
}

pub fn invoke_fn(func: &Value, args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| match func {
        Value::Fn(f) => eval
            .call_fn(f.clone(), args.to_vec())
            .map_err(|e| e.to_string()),
        Value::Native(f) => f(args),
        _ => Err("not a function".to_string()),
    })
}

fn native_gc_collect(_args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| {
        let freed = eval.gc_heap.collect();
        Ok(Value::Int(freed as i64))
    })
}

fn native_gc_count(_args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| Ok(Value::Int(eval.gc_heap.object_count() as i64)))
}

fn native_gc_alloc(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("gc-alloc requires exactly 1 argument".to_string());
    }
    crate::eval::with_evaluator(|eval| {
        let value = args[0].clone();
        let gc = crate::gc::Gc::new(&eval.gc_heap, value);
        Ok(Value::Gc(gc))
    })
}

fn native_gc_deref(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("gc-deref requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Gc(gc) => gc.with(|v| Ok(v.clone())),
        other => Err(format!(
            "gc-deref expects a gc value, got {}",
            other.type_name()
        )),
    }
}

// --- UI DSL (Reagent-like hiccup) ---

const STYLE_KEYS: &[&str] = &[
    "bold", "italic", "underline", "dim", "strikethrough",
    "reverse", "blink", "fg", "color", "bg",
];

fn style_key(kw: &str) -> Value {
    Value::keyword(kw)
}

fn extract_style_from_map(map: &HashMap<Value, Value>) -> Option<Value> {
    let mut pairs = Vec::new();
    for &k in STYLE_KEYS {
        let key = style_key(k);
        if let Some(val) = map.get(&key) {
            let normalized_key = if k == "color" { style_key("fg") } else { key };
            pairs.push((normalized_key, val.clone()));
        }
    }
    if pairs.is_empty() {
        None
    } else {
        Some(Value::map(pairs))
    }
}

fn is_props_map(arg: &Value) -> bool {
    matches!(arg, Value::Map(_))
}

fn get_map(arg: &Value) -> Option<&HashMap<Value, Value>> {
    match arg {
        Value::Map(m) => Some(m.as_ref()),
        _ => None,
    }
}

fn map_get_str(map: &HashMap<Value, Value>, key: &str) -> Option<String> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    })
}

fn map_get_f64(map: &HashMap<Value, Value>, key: &str) -> Option<f64> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    })
}

fn map_get_bool(map: &HashMap<Value, Value>, key: &str) -> Option<bool> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn map_get_u16(map: &HashMap<Value, Value>, key: &str) -> Option<u16> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Int(i) => Some(*i as u16),
        _ => None,
    })
}

fn extract_common_props(map: &HashMap<Value, Value>) -> Vec<(Value, Value)> {
    let mut props = Vec::new();
    for &k in &["gap", "align", "justify", "padding", "width", "height",
                 "flex-grow", "flex-shrink", "border", "title",
                 "min-width", "min-height", "max-width", "max-height",
                 "char", "value", "max", "show-percent", "marker",
                 "scroll-top", "on-click", "on-change"] {
        let key = style_key(k);
        if let Some(val) = map.get(&key) {
            props.push((key, val.clone()));
        }
    }
    props
}

fn make_node(typ: &str, content: Option<String>, style: Option<Value>,
              props: Option<Value>, children: Option<Vec<Value>>) -> Value {
    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword(typ)),
    ];
    if let Some(c) = content {
        pairs.push((Value::keyword("content"), Value::string(c)));
    }
    if let Some(s) = style {
        pairs.push((Value::keyword("style"), s));
    }
    if let Some(p) = props {
        pairs.push((Value::keyword("props"), p));
    }
    if let Some(ch) = children {
        pairs.push((Value::keyword("children"), Value::vector(ch)));
    }
    Value::map(pairs)
}

// (text "content" :bold true :color "#ff0000")
// (text {:bold true :color "#ff0000"} "content")
fn native_ui_text(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return make_node("text", Some(String::new()), None, None, None).into_result();
    }

    let mut style = None;
    let mut content = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        if args.len() > 1 {
            content = args[1].to_string();
        }
    } else {
        content = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
            }
        }
    }

    Ok(make_node("text", Some(content), style, None, None))
}

// (span "content" :bold true :color "#ff0000")
fn native_ui_span(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("span", Some(String::new()), None, None, None));
    }

    let mut style = None;
    let mut content = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        if args.len() > 1 {
            content = args[1].to_string();
        }
    } else {
        content = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
            }
        }
    }

    Ok(make_node("span", Some(content), style, None, None))
}

// (column {:gap 1 :align "center"} child1 child2 ...)
// (column child1 child2 ...)
fn native_ui_column(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("column", None, None, props_val, Some(children)))
}

// (row {:gap 1} child1 child2 ...)
fn native_ui_row(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("row", None, None, props_val, Some(children)))
}

// (box {:title "hi" :border true :padding 1} child1 child2 ...)
fn native_ui_box(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("box", None, None, props_val, Some(children)))
}

// (divider) or (divider "-") or (divider {:char "-"})
fn native_ui_divider(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("divider", None, None, None, None));
    }

    if let Some(map) = get_map(&args[0]) {
        let props = extract_common_props(map);
        Ok(make_node("divider", None, None,
            if props.is_empty() { None } else { Some(Value::map(props)) }, None))
    } else {
        let ch = args[0].to_string().chars().next().unwrap_or('─');
        Ok(make_node("divider", None, None,
            Some(Value::map(vec![(Value::keyword("char"), Value::Char(ch))])), None))
    }
}

// (progress {:value 75 :max 100 :width 20})
fn native_ui_progress(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("progress", None, None, None, None));
    }

    if let Some(map) = get_map(&args[0]) {
        let props = extract_common_props(map);
        let style = extract_style_from_map(map);
        let mut all_props = props;
        if let Some(s) = style {
            all_props.push((Value::keyword("style"), s));
        }
        Ok(make_node("progress", None, None,
            if all_props.is_empty() { None } else { Some(Value::map(all_props)) }, None))
    } else {
        Ok(make_node("progress", None, None, None, None))
    }
}

// (ui-list {:marker :bullet} item1 item2 ...)
fn native_ui_list(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let common = extract_common_props(map);
        let style = extract_style_from_map(map);
        let mut p = common;
        if let Some(s) = style {
            p.push((Value::keyword("style"), s));
        }
        (if p.is_empty() { None } else { Some(Value::map(p)) }, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("list", None, None, props_val, Some(children)))
}

// (show condition child)
fn native_ui_show(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("show requires 2 arguments: condition and child".to_string());
    }

    let when = match &args[0] {
        Value::Bool(b) => *b,
        Value::Nil => false,
        _ => true,
    };

    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword("show")),
        (Value::keyword("when"), Value::Bool(when)),
        (Value::keyword("child"), args[1].clone()),
    ];
    Ok(Value::map(pairs))
}

// (for-each f coll) or (for-each coll f)
fn native_ui_for_each(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("for-each requires 2 arguments: function and collection".to_string());
    }

    // Determine which is fn and which is coll
    let (func, coll) = match (&args[0], &args[1]) {
        (Value::Fn(_), Value::Vector(_) | Value::List(_)) => (&args[0], &args[1]),
        (Value::Vector(_) | Value::List(_), Value::Fn(_)) => (&args[1], &args[0]),
        (Value::Native(_), Value::Vector(_) | Value::List(_)) => (&args[0], &args[1]),
        (Value::Vector(_) | Value::List(_), Value::Native(_)) => (&args[1], &args[0]),
        _ => return Err("for-each requires a function and a collection".to_string()),
    };

    // Store fn and coll as props, actual mapping happens at conversion time
    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword("for")),
        (Value::keyword("func"), func.clone()),
        (Value::keyword("coll"), coll.clone()),
    ];
    Ok(Value::map(pairs))
}

// (scroll-view {:scroll-top 0 :height 10} child)
fn native_ui_scroll_view(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("scroll-view requires at least 1 argument".to_string());
    }

    if let Some(map) = get_map(&args[0]) {
        let common = extract_common_props(map);
        let child = if args.len() > 1 { args[1].clone() } else { Value::Nil };
        let mut pairs = vec![
            (Value::keyword("type"), Value::keyword("scroll-view")),
            (Value::keyword("child"), child),
        ];
        if !common.is_empty() {
            pairs.push((Value::keyword("props"), Value::map(common)));
        }
        Ok(Value::map(pairs))
    } else {
        let mut pairs = vec![
            (Value::keyword("type"), Value::keyword("scroll-view")),
            (Value::keyword("child"), args[0].clone()),
        ];
        Ok(Value::map(pairs))
    }
}

// (button {:on-click handler} label)
fn native_ui_button(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("button", Some(String::new()), None, None, None));
    }

    let mut style = None;
    let mut props_pairs = Vec::new();
    let mut label = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        // Extract on-click and other non-style props
        if let Some(on_click) = map.get(&style_key("on-click")) {
            props_pairs.push((Value::keyword("on-click"), on_click.clone()));
        }
        if args.len() > 1 {
            label = args[1].to_string();
        }
    } else {
        label = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
                if let Some(on_click) = map.get(&style_key("on-click")) {
                    props_pairs.push((Value::keyword("on-click"), on_click.clone()));
                }
            }
        }
    }

    let props = if props_pairs.is_empty() { None } else { Some(Value::map(props_pairs)) };
    Ok(make_node("button", Some(label), style, props, None))
}

trait IntoResult {
    fn into_result(self) -> Result<Value, String>;
}

impl IntoResult for Value {
    fn into_result(self) -> Result<Value, String> {
        Ok(self)
    }
}
