use crate::CompileTarget;

/// Generate Rust source code that embeds and evaluates mora-lisp code
pub fn generate(code: &str, source_name: &str, target: CompileTarget) -> String {
    let escaped_code = escape_string(code);

    match target {
        CompileTarget::Binary => generate_binary(&escaped_code, source_name),
        CompileTarget::SharedLib => generate_shared_lib(&escaped_code, source_name),
    }
}

fn generate_binary(code: &str, source_name: &str) -> String {
    // Build the output manually to avoid format! escaping issues
    let mut out = String::new();
    out.push_str(&format!("//! Auto-generated from {}\n", source_name));
    out.push_str("//! Do not edit manually\n");
    out.push_str("\n");
    out.push_str("use mora_lisp::MoraLisp;\n");
    out.push_str("\n");
    out.push_str("fn main() {\n");
    out.push_str(&format!("    let code = {};\n", code));
    out.push_str("    let mut lisp = MoraLisp::new();\n");
    out.push_str("\n");
    out.push_str("    match lisp.eval(code) {\n");
    out.push_str("        Ok(result) => {\n");
    out.push_str("            match &result {\n");
    out.push_str("                mora_lisp::types::Value::Nil => {}\n");
    out.push_str("                mora_lisp::types::Value::String(s) => println!(\"{}\", s),\n");
    out.push_str("                _ => println!(\"{}\", result),\n");
    out.push_str("            }\n");
    out.push_str("            std::process::exit(0);\n");
    out.push_str("        }\n");
    out.push_str("        Err(e) => {\n");
    out.push_str("            eprintln!(\"Error: {}\", e);\n");
    out.push_str("            std::process::exit(1);\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn generate_shared_lib(_code: &str, source_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("//! Auto-generated from {}\n", source_name));
    out.push_str("//! Do not edit manually\n");
    out.push_str("\n");
    out.push_str("use mora_lisp::MoraLisp;\n");
    out.push_str("use std::ffi::{CStr, CString};\n");
    out.push_str("use std::os::raw::c_char;\n");
    out.push_str("use std::sync::Mutex;\n");
    out.push_str("\n");
    out.push_str("static LISP: Mutex<Option<MoraLisp>> = Mutex::new(None);\n");
    out.push_str("\n");
    out.push_str("/// Initialize the mora-lisp runtime\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub extern \"C\" fn mora_init() {\n");
    out.push_str("    let mut guard = LISP.lock().unwrap();\n");
    out.push_str("    *guard = Some(MoraLisp::new());\n");
    out.push_str("}\n");
    out.push_str("\n");
    out.push_str("/// Evaluate mora-lisp code\n");
    out.push_str("/// Returns a newly allocated string that must be freed with mora_free_string\n");
    out.push_str("/// Returns null on error\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub unsafe extern \"C\" fn mora_eval(code: *const c_char) -> *mut c_char {\n");
    out.push_str("    if code.is_null() {\n");
    out.push_str("        return std::ptr::null_mut();\n");
    out.push_str("    }\n");
    out.push_str("\n");
    out.push_str("    let code_str = match unsafe { CStr::from_ptr(code) }.to_str() {\n");
    out.push_str("        Ok(s) => s,\n");
    out.push_str("        Err(_) => return std::ptr::null_mut(),\n");
    out.push_str("    };\n");
    out.push_str("\n");
    out.push_str("    // Ensure initialized\n");
    out.push_str("    {\n");
    out.push_str("        let guard = LISP.lock().unwrap();\n");
    out.push_str("        if guard.is_none() {\n");
    out.push_str("            drop(guard);\n");
    out.push_str("            mora_init();\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("\n");
    out.push_str("    let mut guard = LISP.lock().unwrap();\n");
    out.push_str("    let lisp = guard.as_mut().unwrap();\n");
    out.push_str("\n");
    out.push_str("    match lisp.eval(code_str) {\n");
    out.push_str("        Ok(result) => {\n");
    out.push_str("            let s = format!(\"{}\", result);\n");
    out.push_str("            match CString::new(s) {\n");
    out.push_str("                Ok(cs) => cs.into_raw(),\n");
    out.push_str("                Err(_) => std::ptr::null_mut(),\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("        Err(e) => {\n");
    out.push_str("            let s = format!(\"Error: {}\", e);\n");
    out.push_str("            match CString::new(s) {\n");
    out.push_str("                Ok(cs) => cs.into_raw(),\n");
    out.push_str("                Err(_) => std::ptr::null_mut(),\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out.push_str("\n");
    out.push_str("/// Free a string returned by mora_eval\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub unsafe extern \"C\" fn mora_free_string(s: *mut c_char) {\n");
    out.push_str("    if !s.is_null() {\n");
    out.push_str("        unsafe { drop(CString::from_raw(s)) };\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out.push_str("\n");
    out.push_str("/// Get the mora-lisp version\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub extern \"C\" fn mora_version() -> *mut c_char {\n");
    out.push_str("    let version = env!(\"CARGO_PKG_VERSION\");\n");
    out.push_str("    CString::new(version).unwrap().into_raw()\n");
    out.push_str("}\n");
    out
}

fn escape_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 2);
    escaped.push('"');
    for c in s.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}
