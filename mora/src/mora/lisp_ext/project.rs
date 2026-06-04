use super::editor_state::with_editor_state;
use super::helpers::extract_string;
use crate::lisp::types::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Find project root by walking up from a starting path, looking for
/// common project markers.
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let markers = [".git", ".projectile", "Makefile", "Cargo.toml"];
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        for marker in &markers {
            if dir.join(marker).exists() {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Get the project root, or the file's parent dir, or cwd as fallback.
fn resolve_project_root() -> Option<PathBuf> {
    with_editor_state(|state| {
        if let Some(ref fp) = state.file_path {
            let p = Path::new(fp);
            find_project_root(p).or_else(|| {
                if p.is_dir() {
                    Some(p.to_path_buf())
                } else {
                    p.parent().map(|d| d.to_path_buf())
                }
            })
        } else {
            std::env::current_dir().ok()
        }
    })
}

/// Recursively collect files up to max_depth, skipping excluded dirs.
fn walk_dir(dir: &Path, results: &mut Vec<String>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str());
            match name {
                Some("node_modules" | "target") => continue,
                _ => {
                    if depth + 1 <= max_depth {
                        walk_dir(&path, results, depth + 1, max_depth);
                    }
                }
            }
        } else if let Some(s) = path.to_str() {
            results.push(s.to_string());
        }
    }
}

/// Recursively find file by name (basename) up to max_depth.
fn find_file_by_name(dir: &Path, filename: &str, depth: usize, max_depth: usize) -> Option<String> {
    if depth > max_depth {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str());
            match name {
                Some("node_modules" | "target") => continue,
                _ => {
                    if let Some(found) = find_file_by_name(&path, filename, depth + 1, max_depth) {
                        return Some(found);
                    }
                }
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            return path.to_str().map(|s| s.to_string());
        }
    }
    None
}

// ── primitives ──────────────────────────────────────────────────────────────

/// (project-root) → project root directory as string, or nil.
fn prim_project_root(_args: &[Value]) -> Result<Value, String> {
    match resolve_project_root() {
        Some(root) => Ok(Value::string(root.to_string_lossy().to_string())),
        None => Ok(Value::Nil),
    }
}

/// (project-name) → basename of project root, or nil.
fn prim_project_name(_args: &[Value]) -> Result<Value, String> {
    match resolve_project_root() {
        Some(root) => {
            let name = root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled");
            Ok(Value::string(name.to_string()))
        }
        None => Ok(Value::Nil),
    }
}

/// (project-files) → vector of all files in project (max depth 5).
fn prim_project_files(_args: &[Value]) -> Result<Value, String> {
    let root =
        resolve_project_root().ok_or_else(|| "project-files: no project root found".to_string())?;
    let mut files = Vec::new();
    walk_dir(&root, &mut files, 0, 5);
    files.sort();
    Ok(Value::Vector(Arc::new(
        files.into_iter().map(Value::string).collect(),
    )))
}

/// (project-find-file NAME) → full path of file by name, or nil.
fn prim_project_find_file(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let root = resolve_project_root()
        .ok_or_else(|| "project-find-file: no project root found".to_string())?;
    match find_file_by_name(&root, &name, 0, 5) {
        Some(path) => Ok(Value::string(path)),
        None => Ok(Value::Nil),
    }
}

/// (project-shell-command CMD) → run shell command in project root.
fn prim_project_shell_command(args: &[Value]) -> Result<Value, String> {
    let cmd = extract_string(args, 0)?;
    let root = resolve_project_root()
        .ok_or_else(|| "project-shell-command: no project root found".to_string())?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("project-shell-command: failed to run: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let trimmed = stdout.trim_end_matches('\n').to_string();
    Ok(Value::string(trimmed))
}

/// (project-grep PATTERN) → search for pattern in project files via grep -rn.
fn prim_project_grep(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    let root =
        resolve_project_root().ok_or_else(|| "project-grep: no project root found".to_string())?;
    let output = std::process::Command::new("grep")
        .args([
            "-rn",
            "--exclude-dir=.git",
            "--exclude-dir=node_modules",
            "--exclude-dir=target",
            &pattern,
            ".",
        ])
        .current_dir(&root)
        .output()
        .map_err(|e| format!("project-grep: failed to run: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let trimmed = stdout.trim_end_matches('\n').to_string();
    if trimmed.is_empty() {
        Ok(Value::Nil)
    } else {
        Ok(Value::string(trimmed))
    }
}

// ── registration ────────────────────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "project-root",
        Value::Native(prim_project_root),
        "Return the current project root directory, or nil.",
    );
    ns.intern_with_doc(
        "project-name",
        Value::Native(prim_project_name),
        "Return the current project name (basename of root).",
    );
    ns.intern_with_doc(
        "project-files",
        Value::Native(prim_project_files),
        "Return a vector of all files in the project (max depth 5).",
    );
    ns.intern_with_doc(
        "project-find-file",
        Value::Native(prim_project_find_file),
        "Find a file by name in the project. Return full path or nil.",
    );
    ns.intern_with_doc(
        "project-shell-command",
        Value::Native(prim_project_shell_command),
        "Run a shell command in the project root. Return stdout.",
    );
    ns.intern_with_doc(
        "project-grep",
        Value::Native(prim_project_grep),
        "Search for PATTERN in project files using grep. Return matches or nil.",
    );
}
