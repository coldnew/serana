use std::process::Command;

use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::helpers::extract_string;

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {}: {}", program, e))?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .to_string()
        .trim_end_matches('\n')
        .to_string())
}

/// (grep PATTERN) — Run grep -rn PATTERN on current directory.
fn prim_grep(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    let output = run_cmd("grep", &["-rn", &pattern, "."])?;
    Ok(Value::string(output))
}

/// (ripgrep PATTERN) — Run rg PATTERN on current directory, fallback to grep.
fn prim_ripgrep(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    match run_cmd("rg", &["--line-number", &pattern]) {
        Ok(out) => Ok(Value::string(out)),
        Err(_) => {
            // Fall back to grep
            let output = run_cmd("grep", &["-rn", &pattern, "."])?;
            Ok(Value::string(output))
        }
    }
}

/// (grep-in-dir PATTERN DIR) — Run grep -rn PATTERN in specified directory.
fn prim_grep_in_dir(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    let dir = extract_string(args, 1)?;
    let output = run_cmd("grep", &["-rn", &pattern, &dir])?;
    Ok(Value::string(output))
}

/// (grep-files PATTERN) — Return vector of file paths matching pattern.
fn prim_grep_files(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    let output = run_cmd("grep", &["-rl", &pattern, "."])?;
    let files: Vec<Value> = output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| Value::string(l.to_string()))
        .collect();
    Ok(Value::vector(files))
}

pub fn register(ns: &mut Namespace) {
    ns.intern_with_doc(
        "grep",
        Value::Native(prim_grep),
        "Run grep -rn PATTERN on current directory.",
    );
    ns.intern_with_doc(
        "ripgrep",
        Value::Native(prim_ripgrep),
        "Run rg PATTERN on current directory, fallback to grep.",
    );
    ns.intern_with_doc(
        "grep-in-dir",
        Value::Native(prim_grep_in_dir),
        "Run grep -rn PATTERN in specified directory.",
    );
    ns.intern_with_doc(
        "grep-files",
        Value::Native(prim_grep_files),
        "Return vector of file paths matching PATTERN.",
    );
    ns.intern_private_with_doc("g", Value::Native(prim_grep), "Alias for grep.");
    ns.intern_private_with_doc("rg", Value::Native(prim_ripgrep), "Alias for ripgrep.");
}
