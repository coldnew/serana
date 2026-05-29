use crate::lisp::types::Value;
use super::editor_state::*;
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("read-string", Value::Native(prim_read_string), "Read a string from the minibuffer, prompting with PROMPT.");
    ns.intern_with_doc("completing-read", Value::Native(prim_completing_read), "Read a string from the minibuffer with completion.");
    ns.intern_with_doc("y-or-n?", Value::Native(prim_y_or_n), "Ask the user a yes-or-no question with PROMPT.");
}

/// (read-string "prompt" ["default"]) → read a string from the minibuffer
fn prim_read_string(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    let default = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });
    // In headless/test mode, return default or empty
    // In interactive mode, this would display the minibuffer prompt
    Ok(Value::string(default.unwrap_or_default()))
}

/// (completing-read "prompt" [choices] ["default"]) → read with completion
fn prim_completing_read(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    let _choices = args.get(1).cloned().unwrap_or(Value::Nil);
    let default = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });
    // In headless/test mode, return default or first choice
    Ok(Value::string(default.unwrap_or_default()))
}

/// (y-or-n? "prompt") → ask yes/no question
fn prim_y_or_n(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    // In headless/test mode, default to true
    Ok(Value::Bool(true))
}
