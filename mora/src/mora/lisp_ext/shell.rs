use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::with_editor_state_mut;
use super::helpers::{extract_int, extract_string};

fn prim_shell_command(args: &[Value]) -> Result<Value, String> {
    let cmd = extract_string(args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("failed to run command: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        let trimmed = stdout.trim_end_matches('\n').to_string();
        with_editor_state_mut(|state| {
            state.status_message = if trimmed.is_empty() {
                format!(
                    "Shell command succeeded (exit {})",
                    output.status.code().unwrap_or(0)
                )
            } else {
                trimmed.clone()
            };
            Ok(Value::string(trimmed))
        })
    } else {
        let msg = if stderr.is_empty() {
            format!(
                "Command failed (exit {})",
                output.status.code().unwrap_or(-1)
            )
        } else {
            stderr.trim_end_matches('\n').to_string()
        };
        with_editor_state_mut(|state| {
            state.status_message = msg.clone();
            Ok(Value::string(msg))
        })
    }
}

fn prim_shell_capture(args: &[Value]) -> Result<Value, String> {
    let cmd = extract_string(args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("failed to run command: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::string(stdout.trim_end_matches('\n').to_string()))
}

pub fn register(ns: &mut Namespace) {
    ns.intern_with_doc("shell-command", Value::Native(prim_shell_command), "Execute COMMAND in a shell and return output.");
    ns.intern_private_with_doc("command", Value::Native(prim_shell_command), "Execute COMMAND in a shell and return output.");
    ns.intern_with_doc("shell-capture", Value::Native(prim_shell_capture), "Execute COMMAND and return its stdout as a string.");
    ns.intern_private_with_doc("capture", Value::Native(prim_shell_capture), "Execute COMMAND and return its stdout as a string.");
}
