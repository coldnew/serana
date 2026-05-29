use crate::lisp::types::Value;

pub fn extract_string(args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(v) => Err(format!("expected string, got {:?}", v)),
        None => Err("missing argument".to_string()),
    }
}

pub fn extract_int(args: &[Value], idx: usize) -> Result<i64, String> {
    match args.get(idx) {
        Some(Value::Int(n)) => Ok(*n),
        Some(v) => Err(format!("expected int, got {:?}", v)),
        None => Err("missing argument".to_string()),
    }
}
