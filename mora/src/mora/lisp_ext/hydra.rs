/// Hydra — make key bindings that stick around.
///
/// Inspired by abo-abo/hydra for Emacs.
/// After pressing a prefix key, a popup shows available bindings.
/// Pressing a listed key executes the action and the popup stays open
/// (for "continuing" heads) or closes (for "exiting" heads).
///
/// Colors:
///   amaranth — non-heads blocked, heads continue
///   teal     — non-heads blocked, heads exit
///   pink     — non-heads allowed, heads continue
///   red      — non-heads allowed then exit, heads continue
///   blue     — non-heads allowed then exit, heads exit

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

// ── Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum HydraColor {
    Amaranth,
    Teal,
    Pink,
    Red,
    Blue,
}

impl HydraColor {
    pub fn from_str(s: &str) -> Self {
        match s {
            "amaranth" => HydraColor::Amaranth,
            "teal" => HydraColor::Teal,
            "pink" => HydraColor::Pink,
            "red" => HydraColor::Red,
            "blue" | _ => HydraColor::Blue,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HydraHead {
    pub key: String,
    pub action: Value,
    pub hint: String,
    pub color: Option<HydraColor>,
}

#[derive(Debug, Clone)]
pub struct Hydra {
    pub name: String,
    pub body_color: HydraColor,
    pub docstring: String,
    pub heads: Vec<HydraHead>,
}

/// What a hydra head color resolves to: continue or exit after execution.
#[derive(Debug, Clone, PartialEq)]
enum HeadBehavior {
    Continue,
    Exit,
}

/// What happens when a non-head key is pressed.
#[derive(Debug, Clone, PartialEq)]
enum NonHeadBehavior {
    Block,    // key press ignored
    AllowAndContinue,
    AllowAndExit,
}

impl HydraColor {
    fn default_head_behavior(&self) -> HeadBehavior {
        match self {
            HydraColor::Amaranth | HydraColor::Pink | HydraColor::Red => HeadBehavior::Continue,
            HydraColor::Teal | HydraColor::Blue => HeadBehavior::Exit,
        }
    }

    fn non_head_behavior(&self) -> NonHeadBehavior {
        match self {
            HydraColor::Amaranth | HydraColor::Teal => NonHeadBehavior::Block,
            HydraColor::Pink => NonHeadBehavior::AllowAndContinue,
            HydraColor::Red => NonHeadBehavior::AllowAndExit,
            HydraColor::Blue => NonHeadBehavior::AllowAndExit,
        }
    }
}

// ── Global hydra registry ────────────────────────────────────

thread_local! {
    static HYDRA_REGISTRY: RefCell<HashMap<String, Hydra>> = RefCell::new(HashMap::new());
    static ACTIVE_HYDRA: RefCell<Option<String>> = RefCell::new(None);
}

pub fn register_hydra(name: String, hydra: Hydra) {
    HYDRA_REGISTRY.with(|reg| {
        reg.borrow_mut().insert(name, hydra);
    });
}

pub fn get_hydra(name: &str) -> Option<Hydra> {
    HYDRA_REGISTRY.with(|reg| reg.borrow().get(name).cloned())
}

pub fn set_active(name: Option<String>) {
    ACTIVE_HYDRA.with(|a| {
        *a.borrow_mut() = name;
    });
}

pub fn active_hydra_name() -> Option<String> {
    ACTIVE_HYDRA.with(|a| a.borrow().clone())
}

pub fn is_active() -> bool {
    ACTIVE_HYDRA.with(|a| a.borrow().is_some())
}

pub fn hydra_names() -> Vec<String> {
    HYDRA_REGISTRY.with(|reg| {
        let mut names: Vec<String> = reg.borrow().keys().cloned().collect();
        names.sort();
        names
    })
}

// ── Hint generation ──────────────────────────────────────────

fn head_behavior(hydra: &Hydra, head: &HydraHead) -> HeadBehavior {
    match &head.color {
        Some(c) => c.default_head_behavior(),
        None => hydra.body_color.default_head_behavior(),
    }
}

fn format_hint(hydra: &Hydra) -> String {
    let mut parts = Vec::new();
    parts.push(format!("{}:", hydra.name));

    for head in &hydra.heads {
        let behavior = head_behavior(hydra, head);
        let marker = match behavior {
            HeadBehavior::Continue => "",
            HeadBehavior::Exit => "*",
        };
        let hint = if head.hint.is_empty() {
            head.key.clone()
        } else {
            head.hint.clone()
        };
        parts.push(format!("[{}]{} {}", head.key, marker, hint));
    }

    // Add exit hint
    let has_explicit_exit = hydra.heads.iter().any(|h| {
        head_behavior(hydra, h) == HeadBehavior::Exit
    });
    if !has_explicit_exit {
        parts.push("[q] quit".to_string());
    }

    parts.join(" ")
}

// ── Lisp primitives ──────────────────────────────────────────

/// (defhydra NAME (:color COLOR) DOCSTRING (KEY ACTION HINT [:color COLOR]) ...)
/// Define a new hydra.
fn prim_defhydra(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("defhydra requires at least name, options, and docstring".to_string());
    }

    // Parse name
    let name = match &args[0] {
        Value::Symbol(s) => s.name.to_string(),
        Value::String(s) => s.to_string(),
        _ => return Err("defhydra name must be a symbol or string".to_string()),
    };

    // Parse options map
    let body_color = match &args[1] {
        Value::Map(map) => {
            let color_val = map.get(&Value::keyword("color"));
            match color_val {
                Some(Value::Keyword(k)) => HydraColor::from_str(k.name.as_str()),
                Some(Value::String(s)) => HydraColor::from_str(s),
                _ => HydraColor::Red,
            }
        }
        _ => HydraColor::Red,
    };

    // Parse docstring
    let docstring = match &args[2] {
        Value::String(s) => s.to_string(),
        _ => String::new(),
    };

    // Parse heads (remaining args are head specs)
    let mut heads = Vec::new();
    for head_spec in args.iter().skip(3) {
        match head_spec {
            Value::Vector(v) if v.len() >= 2 => {
                let key = match &v[0] {
                    Value::String(s) => s.to_string(),
                    Value::Char(c) => c.to_string(),
                    _ => continue,
                };

                let action = v[1].clone();

                let hint = if v.len() >= 3 {
                    match &v[2] {
                        Value::String(s) => s.to_string(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                };

                let head_color = if v.len() >= 5 {
                    match &v[3] {
                        Value::Keyword(k) if k.name.as_str() == "color" => {
                            match &v[4] {
                                Value::Keyword(c) => Some(HydraColor::from_str(c.name.as_str())),
                                Value::String(s) => Some(HydraColor::from_str(s)),
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                heads.push(HydraHead {
                    key,
                    action,
                    hint,
                    color: head_color,
                });
            }
            Value::List(v) if v.len() >= 2 => {
                // Same parsing for lists
                let key = match &v[0] {
                    Value::String(s) => s.to_string(),
                    Value::Char(c) => c.to_string(),
                    _ => continue,
                };
                let action = v[1].clone();
                let hint = v.get(2).and_then(|v| match v {
                    Value::String(s) => Some(s.to_string()),
                    _ => None,
                }).unwrap_or_default();
                heads.push(HydraHead {
                    key,
                    action,
                    hint,
                    color: None,
                });
            }
            _ => {}
        }
    }

    if heads.is_empty() {
        return Err("defhydra requires at least one head".to_string());
    }

    let hydra = Hydra {
        name: name.clone(),
        body_color,
        docstring,
        heads,
    };

    register_hydra(name.clone(), hydra);
    Ok(Value::string(name))
}

/// (hydra-hint NAME) → return hint string for the named hydra
fn prim_hydra_hint(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let hydra = get_hydra(&name).ok_or_else(|| format!("hydra not found: {}", name))?;
    Ok(Value::string(format_hint(&hydra)))
}

/// (hydra-exit) → exit the currently active hydra
fn prim_hydra_exit(_args: &[Value]) -> Result<Value, String> {
    set_active(None);
    Ok(Value::Nil)
}

/// (hydra-active?) → return name of active hydra or nil
fn prim_hydra_active(_args: &[Value]) -> Result<Value, String> {
    match active_hydra_name() {
        Some(name) => Ok(Value::string(name)),
        None => Ok(Value::Nil),
    }
}

/// (hydra-names) → return vector of all defined hydra names
fn prim_hydra_names(_args: &[Value]) -> Result<Value, String> {
    let names = hydra_names();
    Ok(Value::vector(
        names.into_iter().map(Value::string).collect(),
    ))
}

/// (hydra-call-head NAME KEY) → execute the head with KEY in hydra NAME.
/// Returns [result should-continue] where should-continue is bool.
fn prim_hydra_call_head(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let key = extract_string(args, 1)?;

    let hydra = get_hydra(&name).ok_or_else(|| format!("hydra not found: {}", name))?;

    // Find matching head
    let head = hydra.heads.iter().find(|h| h.key == key);
    match head {
        Some(head) => {
            let behavior = head_behavior(&hydra, head);

            // Execute the action
            let result = match &head.action {
                Value::Nil => Value::Nil,
                Value::Fn(f) => {
                    crate::lisp::eval::with_evaluator(|eval| {
                        eval.call_fn(f.clone(), vec![]).map_err(|e| e.to_string())
                    })?
                }
                Value::Native(f) => f(&[])?,
                Value::Symbol(_) | Value::String(_) => {
                    // Execute as command name (placeholder)
                    Value::Nil
                }
                _ => Value::Nil,
            };

            let should_continue = behavior == HeadBehavior::Continue;

            if should_continue {
                set_active(Some(name.clone()));
            } else {
                set_active(None);
            }

            Ok(Value::vector(vec![
                result,
                Value::Bool(should_continue),
            ]))
        }
        None => {
            // Not a head — check non-head behavior
            let nhb = hydra.body_color.non_head_behavior();
            match nhb {
                NonHeadBehavior::Block => {
                    // Keep hydra active, ignore key
                    set_active(Some(name.clone()));
                    Ok(Value::vector(vec![
                        Value::Nil,
                        Value::Bool(true), // continue
                    ]))
                }
                NonHeadBehavior::AllowAndContinue => {
                    set_active(Some(name.clone()));
                    Ok(Value::vector(vec![
                        Value::Nil,
                        Value::Bool(true),
                    ]))
                }
                NonHeadBehavior::AllowAndExit => {
                    set_active(None);
                    Ok(Value::vector(vec![
                        Value::Nil,
                        Value::Bool(false),
                    ]))
                }
            }
        }
    }
}

/// (hydra-define NAME (:color COLOR) DOC HEADS) → define hydra from Lisp
/// where HEADS is a vector of [key action hint] vectors.
fn prim_hydra_define(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;

    let body_color = match &args.get(1) {
        Some(Value::Map(map)) => {
            map.get(&Value::keyword("color"))
                .and_then(|v| match v {
                    Value::Keyword(k) => Some(HydraColor::from_str(k.name.as_str())),
                    Value::String(s) => Some(HydraColor::from_str(s)),
                    _ => None,
                })
                .unwrap_or(HydraColor::Red)
        }
        _ => HydraColor::Red,
    };

    let docstring = args.get(2)
        .and_then(|v| match v { Value::String(s) => Some(s.to_string()), _ => None })
        .unwrap_or_default();

    let heads_val = args.get(3).ok_or("hydra-define requires heads vector")?;
    let heads_vec = match heads_val {
        Value::Vector(v) => v.as_ref().clone(),
        Value::List(v) => v.as_ref().clone(),
        _ => return Err("heads must be a vector".to_string()),
    };

    let mut heads = Vec::new();
    for head_spec in heads_vec {
        match head_spec {
            Value::Vector(v) if v.len() >= 2 => {
                let key = match &v[0] {
                    Value::String(s) => s.to_string(),
                    Value::Char(c) => c.to_string(),
                    _ => continue,
                };
                let action = v[1].clone();
                let hint = v.get(2).and_then(|v| match v {
                    Value::String(s) => Some(s.to_string()),
                    _ => None,
                }).unwrap_or_default();
                let head_color = if v.len() >= 5 {
                    match (&v[3], &v[4]) {
                        (Value::Keyword(k), Value::Keyword(c)) if k.name.as_str() == "color" => {
                            Some(HydraColor::from_str(c.name.as_str()))
                        }
                        (Value::Keyword(k), Value::String(s)) if k.name.as_str() == "color" => {
                            Some(HydraColor::from_str(s.as_str()))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                heads.push(HydraHead { key, action, hint, color: head_color });
            }
            _ => {}
        }
    }

    if heads.is_empty() {
        return Err("hydra requires at least one head".to_string());
    }

    let hydra = Hydra {
        name: name.clone(),
        body_color,
        docstring,
        heads,
    };

    register_hydra(name.clone(), hydra);
    Ok(Value::string(name))
}

/// (hydra-info NAME) → return map with hydra details
fn prim_hydra_info(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let hydra = get_hydra(&name).ok_or_else(|| format!("hydra not found: {}", name))?;

    let head_pairs: Vec<(Value, Value)> = hydra.heads.iter().map(|h| {
        let info = Value::vector(vec![
            Value::string(&h.key),
            Value::string(&h.hint),
            Value::string(match &h.color {
                Some(HydraColor::Red) => "red",
                Some(HydraColor::Blue) => "blue",
                None => "inherit",
                _ => "inherit",
            }),
        ]);
        (Value::string(&h.key), info)
    }).collect();

    let mut pairs = vec![
        (Value::keyword("name"), Value::string(&hydra.name)),
        (Value::keyword("docstring"), Value::string(&hydra.docstring)),
        (Value::keyword("color"), Value::string(match hydra.body_color {
            HydraColor::Amaranth => "amaranth",
            HydraColor::Teal => "teal",
            HydraColor::Pink => "pink",
            HydraColor::Red => "red",
            HydraColor::Blue => "blue",
        })),
        (Value::keyword("hint"), Value::string(format_hint(&hydra))),
        (Value::keyword("head-count"), Value::Int(hydra.heads.len() as i64)),
    ];

    Ok(Value::map(pairs))
}

// ── Registration ─────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("defhydra", Value::Native(prim_defhydra),
        "Define a hydra: (defhydra NAME {:color COLOR} DOCSTRING (KEY ACTION HINT) ...)");
    ns.intern_with_doc("hydra-hint", Value::Native(prim_hydra_hint),
        "Return the hint string for hydra NAME.");
    ns.intern_with_doc("hydra-exit", Value::Native(prim_hydra_exit),
        "Exit the currently active hydra.");
    ns.intern_with_doc("hydra-active?", Value::Native(prim_hydra_active),
        "Return name of active hydra, or nil.");
    ns.intern_with_doc("hydra-names", Value::Native(prim_hydra_names),
        "Return vector of all defined hydra names.");
    ns.intern_with_doc("hydra-call-head", Value::Native(prim_hydra_call_head),
        "Execute head KEY in hydra NAME. Returns [result continue?].");
    ns.intern_with_doc("hydra-define", Value::Native(prim_hydra_define),
        "Define hydra from a vector of heads: (hydra-NAME {:color COLOR} DOC HEADS).");
    ns.intern_with_doc("hydra-info", Value::Native(prim_hydra_info),
        "Return info map for hydra NAME.");
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
        HYDRA_REGISTRY.with(|r| r.borrow_mut().clear());
        set_active(None);
    }

    fn make_bridge() -> super::super::core::MoraLispBridge {
        super::super::core::MoraLispBridge::new()
    }

    #[test]
    fn test_defhydra_basic() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "zoom" {:color "pink"} "zoom" [["g" nil "in"] ["l" nil "out"]])"#).unwrap();

        let hint = bridge.eval("(hydra-hint \"zoom\")").unwrap();
        match hint {
            Value::String(s) => {
                assert!(s.contains("zoom"), "should contain docstring: {}", s);
                assert!(s.contains("[g]"), "should show g: {}", s);
                assert!(s.contains("[l]"), "should show l: {}", s);
            }
            _ => panic!("expected string"),
        }
        teardown();
    }

    #[test]
    fn test_hydra_blue_head_exits() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "toggle" {:color "blue"} "toggle" [["a" nil "abbrev"] ["f" nil "fill"]])"#).unwrap();

        let result = bridge.eval("(hydra-call-head \"toggle\" \"a\")").unwrap();
        match result {
            Value::Vector(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[1], Value::Bool(false)); // blue = exit
            }
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_hydra_pink_head_continues() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "nav" {:color "pink"} "navigate" [["h" nil "left"] ["j" nil "down"] ["k" nil "up"] ["l" nil "right"]])"#).unwrap();

        let result = bridge.eval("(hydra-call-head \"nav\" \"h\")").unwrap();
        match result {
            Value::Vector(v) => {
                assert_eq!(v[1], Value::Bool(true)); // pink = continue
            }
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_hydra_amaranth_blocks_non_heads() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "vi" {:color "amaranth"} "vi" [["h" nil "left"] ["j" nil "down"] ["k" nil "up"] ["l" nil "right"]])"#).unwrap();

        // Non-head key: amaranth blocks, so continues
        let result = bridge.eval("(hydra-call-head \"vi\" \"x\")").unwrap();
        match result {
            Value::Vector(v) => {
                assert_eq!(v[1], Value::Bool(true)); // blocked, continues
            }
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_hydra_info() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "test" {:color "red"} "test doc" [["a" nil "alpha"] ["b" nil "beta"]])"#).unwrap();

        let info = bridge.eval("(hydra-info \"test\")").unwrap();
        match info {
            Value::Map(m) => {
                assert_eq!(m.get(&Value::keyword("name")).unwrap(), &Value::string("test"));
                assert_eq!(m.get(&Value::keyword("color")).unwrap(), &Value::string("red"));
                assert_eq!(m.get(&Value::keyword("head-count")).unwrap(), &Value::Int(2));
            }
            _ => panic!("expected map"),
        }
        teardown();
    }

    #[test]
    fn test_hydra_exit() {
        setup();
        let mut bridge = make_bridge();

        assert_eq!(bridge.eval("(hydra-active?)").unwrap(), Value::Nil);

        bridge.eval(r#"(hydra-define "nav2" {:color "pink"} "nav" [["h" nil "left"]])"#).unwrap();
        bridge.eval("(hydra-call-head \"nav2\" \"h\")").unwrap();
        assert!(bridge.eval("(hydra-active?)").unwrap() != Value::Nil);

        bridge.eval("(hydra-exit)").unwrap();
        assert_eq!(bridge.eval("(hydra-active?)").unwrap(), Value::Nil);

        teardown();
    }

    #[test]
    fn test_hydra_names_lists_all() {
        setup();
        let mut bridge = make_bridge();

        bridge.eval(r#"(hydra-define "foo" {:color "red"} "foo" [["a" nil "alpha"]])"#).unwrap();
        bridge.eval(r#"(hydra-define "bar" {:color "blue"} "bar" [["b" nil "beta"]])"#).unwrap();

        let names = bridge.eval("(hydra-names)").unwrap();
        match names {
            Value::Vector(v) => {
                assert_eq!(v.len(), 2);
            }
            _ => panic!("expected vector"),
        }
        teardown();
    }
}
