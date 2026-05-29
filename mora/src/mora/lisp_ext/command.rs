use std::cell::RefCell;
use std::collections::HashMap;

use crate::lisp::types::Value;

use super::helpers::extract_string;

#[derive(Clone)]
pub struct CommandEntry {
    pub func: Value,
    pub doc: Option<String>,
}

thread_local! {
    pub static COMMAND_REGISTRY: RefCell<HashMap<String, CommandEntry>> = RefCell::new(HashMap::new());
}

pub fn clear_command_registry() {
    COMMAND_REGISTRY.with(|registry| registry.borrow_mut().clear());
}

pub fn short_command_name(name: &str) -> &str {
    name.rsplit_once('/')
        .map(|(_, short)| short)
        .unwrap_or(name)
}

pub fn command_names() -> Vec<String> {
    COMMAND_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let mut names: Vec<String> = registry.keys().cloned().collect();
        let mut short_counts = HashMap::new();
        for name in registry.keys() {
            *short_counts
                .entry(short_command_name(name).to_string())
                .or_insert(0) += 1;
        }
        for name in registry.keys() {
            let short = short_command_name(name);
            if short_counts.get(short) == Some(&1) {
                names.push(short.to_string());
            }
        }
        names.sort();
        names.dedup();
        names
    })
}

pub fn command_entry(name: &str) -> Option<CommandEntry> {
    COMMAND_REGISTRY.with(|registry| registry.borrow().get(name).cloned())
}

pub fn resolve_command_name(name: &str) -> Option<String> {
    COMMAND_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        if registry.contains_key(name) {
            return Some(name.to_string());
        }
        let mut matches: Vec<String> = registry
            .keys()
            .filter(|k| short_command_name(k) == name)
            .cloned()
            .collect();
        if matches.len() == 1 {
            return Some(matches.remove(0));
        }
        None
    })
}

pub fn command_doc(name: &str) -> Option<String> {
    resolve_command_name(name)
        .and_then(|name| command_entry(&name).and_then(|entry| entry.doc))
}

fn prim_command_register(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let func = args
        .get(1)
        .cloned()
        .ok_or_else(|| "register! requires a command function".to_string())?;
    if !matches!(func, Value::Fn(_) | Value::Native(_)) {
        return Err(format!(
            "command function must be callable, got {}",
            func.type_name()
        ));
    }
    let doc = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });

    COMMAND_REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .insert(name.clone(), CommandEntry { func, doc });
    });
    Ok(Value::string(name))
}

fn prim_command_execute(args: &[Value]) -> Result<Value, String> {
    let requested = extract_string(args, 0)?;
    let name = resolve_command_name(&requested)
        .ok_or_else(|| format!("command not found or ambiguous: {}", requested))?;
    let entry = command_entry(&name).ok_or_else(|| format!("command not found: {}", name))?;

    crate::lisp::eval::with_evaluator(|eval| match entry.func {
        Value::Fn(f) => eval.call_fn(f, vec![]).map_err(|e| e.to_string()),
        Value::Native(f) => f(&[]),
        other => Err(format!("command {} is {}", name, other.type_name())),
    })
}

fn prim_command_exists(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    Ok(Value::Bool(resolve_command_name(&name).is_some()))
}

fn prim_command_names(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::vector(
        command_names().into_iter().map(Value::string).collect(),
    ))
}

fn prim_command_doc(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    Ok(command_doc(&name).map(Value::string).unwrap_or(Value::Nil))
}
/// (describe-function "name") → return doc string for any function
fn prim_describe_function(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    // Check command registry first
    if let Some(doc) = command_doc(&name) {
        return Ok(Value::string(doc));
    }
    // Check global doc registry (populated by intern_with_doc)
    if let Some(doc) = crate::lisp::ns::lookup_doc(&name) {
        return Ok(Value::string(doc));
    }
    Ok(Value::Nil)
}
pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("register!", Value::Native(prim_command_register), "Register NAME as an interactive command with FUNC.");
    ns.intern_with_doc("execute!", Value::Native(prim_command_execute), "Execute the command NAME.");
    ns.intern_with_doc("exists?", Value::Native(prim_command_exists), "Return t if the command NAME exists.");
    ns.intern_with_doc("names", Value::Native(prim_command_names), "Return a vector of all command names.");
    ns.intern_with_doc("doc", Value::Native(prim_command_doc), "Return the doc string for command NAME.");
    ns.intern_with_doc("describe-function", Value::Native(prim_describe_function), "Return the doc string for the function NAME.");
}
