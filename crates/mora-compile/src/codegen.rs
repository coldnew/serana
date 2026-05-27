use crate::CompileTarget;
use mora_lisp::types::Value;

/// Generate Rust source code that embeds and evaluates mora-lisp code
pub fn generate(code: &str, source_name: &str, target: CompileTarget) -> String {
    match target {
        CompileTarget::Binary => generate_binary(code, source_name),
        CompileTarget::SharedLib => generate_shared_lib(code, source_name),
    }
}

fn generate_binary(code: &str, source_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("//! Auto-generated from {}\n", source_name));
    out.push_str("//! Do not edit manually\n");
    out.push_str("\n");
    out.push_str("use mora_lisp::MoraLisp;\n");
    out.push_str("use mora_lisp::types::Value;\n");
    out.push_str("\n");
    out.push_str("fn main() {\n");

    match mora_lisp::reader::read_all(code) {
        Ok(forms) if !forms.is_empty() => {
            out.push_str("    let mut lisp = MoraLisp::new();\n");
            out.push_str("    let forms: Vec<Value> = vec![\n");
            for form in &forms {
                out.push_str("        ");
                value_to_constructor(&mut out, form);
                out.push_str(",\n");
            }
            out.push_str("    ];\n");
            out.push_str("\n");
            out.push_str("    let mut result = Value::Nil;\n");
            out.push_str("    for form in &forms {\n");
            out.push_str("        match lisp.eval_form(form) {\n");
            out.push_str("            Ok(r) => result = r,\n");
            out.push_str("            Err(e) => {\n");
            out.push_str("                eprintln!(\"Error: {}\", e);\n");
            out.push_str("                std::process::exit(1);\n");
            out.push_str("            }\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("\n");
            out.push_str("    match &result {\n");
            out.push_str("        Value::Nil => {}\n");
            out.push_str("        Value::String(s) => println!(\"{}\", s),\n");
            out.push_str("        _ => println!(\"{}\", result),\n");
            out.push_str("    }\n");
        }
        _ => {
            // Fallback: embed source as string for unparseable code
            let escaped_code = escape_string(code);
            out.push_str(&format!("    let code = {};\n", escaped_code));
            out.push_str("    let mut lisp = MoraLisp::new();\n");
            out.push_str("    match lisp.eval(code) {\n");
            out.push_str("        Ok(result) => {\n");
            out.push_str("            match &result {\n");
            out.push_str("                Value::Nil => {}\n");
            out.push_str("                Value::String(s) => println!(\"{}\", s),\n");
            out.push_str("                _ => println!(\"{}\", result),\n");
            out.push_str("            }\n");
            out.push_str("            std::process::exit(0);\n");
            out.push_str("        }\n");
            out.push_str("        Err(e) => {\n");
            out.push_str("            eprintln!(\"Error: {}\", e);\n");
            out.push_str("            std::process::exit(1);\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
        }
    }

    out.push_str("}\n");
    out
}

fn generate_shared_lib(code: &str, source_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("//! Auto-generated from {}\n", source_name));
    out.push_str("//! Do not edit manually\n");
    out.push_str("\n");
    out.push_str("use mora_lisp::MoraLisp;\n");
    out.push_str("use mora_lisp::types::Value;\n");
    out.push_str("use std::ffi::{CStr, CString};\n");
    out.push_str("use std::os::raw::c_char;\n");
    out.push_str("use std::sync::Mutex;\n");
    out.push_str("\n");
    out.push_str("static LISP: Mutex<Option<MoraLisp>> = Mutex::new(None);\n");
    out.push_str("static FORMS: Mutex<Option<Vec<Value>>> = Mutex::new(None);\n");
    out.push_str("\n");

    // Generate the pre-parsed forms as a lazy init function
    out.push_str("fn init_forms() -> &'static Vec<Value> {\n");
    out.push_str("    static FORMS: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();\n");
    out.push_str("    FORMS.get_or_init(|| {\n");
    out.push_str("        vec![\n");

    match mora_lisp::reader::read_all(code) {
        Ok(forms) if !forms.is_empty() => {
            for form in &forms {
                out.push_str("            ");
                value_to_constructor(&mut out, form);
                out.push_str(",\n");
            }
        }
        _ => {
            // Fallback: parse at runtime
            let escaped_code = escape_string(code);
            out.push_str(&format!(
                "            // Fallback: source could not be pre-parsed\n"
            ));
            out.push_str(&format!(
                "            // Source: {}\n", escaped_code
            ));
        }
    }

    out.push_str("        ]\n");
    out.push_str("    })\n");
    out.push_str("}\n");
    out.push_str("\n");

    // mora_init
    out.push_str("/// Initialize the mora-lisp runtime\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub extern \"C\" fn mora_init() {\n");
    out.push_str("    let mut guard = LISP.lock().unwrap();\n");
    out.push_str("    *guard = Some(MoraLisp::new());\n");
    out.push_str("}\n");
    out.push_str("\n");

    // mora_eval
    out.push_str("/// Evaluate mora-lisp code\n");
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

    // mora_run_forms — runs pre-parsed forms
    out.push_str("/// Run pre-parsed forms (skips parsing)\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub unsafe extern \"C\" fn mora_run_forms() -> *mut c_char {\n");
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
    out.push_str("    let forms = init_forms();\n");
    out.push_str("\n");
    out.push_str("    let mut result = Value::Nil;\n");
    out.push_str("    for form in forms {\n");
    out.push_str("        match lisp.eval_form(form) {\n");
    out.push_str("            Ok(r) => result = r,\n");
    out.push_str("            Err(e) => {\n");
    out.push_str("                let s = format!(\"Error: {}\", e);\n");
    out.push_str("                return CString::new(s).unwrap_or_default().into_raw();\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    let s = format!(\"{}\", result);\n");
    out.push_str("    CString::new(s).unwrap_or_default().into_raw()\n");
    out.push_str("}\n");
    out.push_str("\n");

    // mora_free_string
    out.push_str("/// Free a string returned by mora_eval\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub unsafe extern \"C\" fn mora_free_string(s: *mut c_char) {\n");
    out.push_str("    if !s.is_null() {\n");
    out.push_str("        unsafe { drop(CString::from_raw(s)) };\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out.push_str("\n");

    // mora_version
    out.push_str("/// Get the mora-lisp version\n");
    out.push_str("#[no_mangle]\n");
    out.push_str("pub extern \"C\" fn mora_version() -> *mut c_char {\n");
    out.push_str("    let version = env!(\"CARGO_PKG_VERSION\");\n");
    out.push_str("    CString::new(version).unwrap().into_raw()\n");
    out.push_str("}\n");

    out
}

/// Convert a Value into Rust source code that constructs it
fn value_to_constructor(out: &mut String, v: &Value) {
    match v {
        Value::Nil => out.push_str("Value::Nil"),
        Value::Bool(b) => out.push_str(&format!("Value::Bool({})", b)),
        Value::Int(n) => out.push_str(&format!("Value::Int({})", n)),
        Value::Float(f) => out.push_str(&format!("Value::Float({}f64)", f)),
        Value::String(s) => {
            out.push_str("Value::string(\"");
            out.push_str(&escape_inner(s));
            out.push_str("\")");
        }
        Value::Char(c) => out.push_str(&format!("Value::Char('{}')", escape_char(*c))),
        Value::Keyword(k) => {
            let s = if let Some(ns) = &k.ns {
                format!(":{}/{}", ns, k.name)
            } else {
                format!(":{}", k.name)
            };
            out.push_str("Value::keyword(\"");
            out.push_str(&escape_inner(&s));
            out.push_str("\")");
        }
        Value::Symbol(s) => {
            let name = if let Some(ns) = &s.ns {
                format!("{}/{}", ns, s.name)
            } else {
                s.name.to_string()
            };
            out.push_str("Value::symbol(\"");
            out.push_str(&escape_inner(&name));
            out.push_str("\")");
        }
        Value::List(vals) => {
            out.push_str("Value::list(vec![");
            for (i, val) in vals.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                value_to_constructor(out, val);
            }
            out.push_str("])");
        }
        Value::Vector(vals) => {
            out.push_str("Value::vector(vec![");
            for (i, val) in vals.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                value_to_constructor(out, val);
            }
            out.push_str("])");
        }
        Value::Map(m) => {
            out.push_str("Value::map(vec![");
            for (i, (k, val)) in m.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('(');
                value_to_constructor(out, k);
                out.push_str(", ");
                value_to_constructor(out, val);
                out.push(')');
            }
            out.push_str("])");
        }
        Value::Set(vals) => {
            out.push_str("Value::set(vec![");
            for (i, val) in vals.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                value_to_constructor(out, val);
            }
            out.push_str("])");
        }
        // Runtime-only values — shouldn't appear in parsed source
        _ => out.push_str("Value::Nil"),
    }
}

fn escape_char(c: char) -> String {
    match c {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        _ => c.to_string(),
    }
}

fn escape_inner(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
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
    escaped
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
