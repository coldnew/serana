use crate::lisp::types::Value;

use super::helpers::extract_string;

use super::super::tramp as tramp_mod;

fn parse_tramp_path_arg(args: &[Value], idx: usize) -> Result<tramp_mod::RemotePath, String> {
    let path_str = extract_string(args, idx)?;
    tramp_mod::RemotePath::parse(&path_str)
        .ok_or_else(|| format!("invalid TRAMP path: {}", path_str))
}

fn prim_tramp_read_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let content = tramp_mod::read_file(&rp)?;
    Ok(Value::string(content))
}

fn prim_tramp_write_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let content = extract_string(args, 1)?;
    tramp_mod::write_file(&rp, &content)?;
    Ok(Value::Nil)
}

fn prim_tramp_shell_command(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let cmd = extract_string(args, 1)?;
    let (stdout, code) = tramp_mod::shell_command(&rp, &cmd)?;
    Ok(Value::vector(vec![
        Value::string(stdout),
        Value::Int(code as i64),
    ]))
}

fn prim_tramp_shell_capture(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let cmd = extract_string(args, 1)?;
    let stdout = tramp_mod::shell_capture(&rp, &cmd)?;
    Ok(Value::string(stdout))
}

fn prim_tramp_file_exists(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let exists = tramp_mod::file_exists(&rp)?;
    Ok(Value::Bool(exists))
}

fn prim_tramp_file_mtime(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    match tramp_mod::file_mtime(&rp) {
        Ok(mtime) => Ok(Value::Int(mtime)),
        Err(_) => Ok(Value::Nil),
    }
}

fn prim_tramp_list_dir(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let entries = tramp_mod::list_directory(&rp)?;
    Ok(Value::vector(
        entries.into_iter().map(Value::string).collect(),
    ))
}

fn prim_tramp_mkdir(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    tramp_mod::make_directory(&rp)?;
    Ok(Value::Nil)
}

fn prim_tramp_delete_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    tramp_mod::delete_file(&rp)?;
    Ok(Value::Nil)
}

fn prim_tramp_rename_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let new_path = extract_string(args, 1)?;
    tramp_mod::rename_file(&rp, &new_path)?;
    Ok(Value::Nil)
}

fn prim_tramp_ping(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let ok = tramp_mod::ping(&rp)?;
    Ok(Value::Bool(ok))
}

fn prim_tramp_connect(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    tramp_mod::pool().touch(&rp.ssh_target());
    Ok(Value::string(rp.ssh_target()))
}

fn prim_tramp_disconnect(args: &[Value]) -> Result<Value, String> {
    let target = extract_string(args, 0)?;
    tramp_mod::pool().disconnect(&target);
    Ok(Value::Nil)
}

fn prim_tramp_connections(_args: &[Value]) -> Result<Value, String> {
    let conns = tramp_mod::pool().list();
    Ok(Value::vector(
        conns.into_iter().map(Value::string).collect(),
    ))
}

fn prim_tramp_parse_path(args: &[Value]) -> Result<Value, String> {
    let path_str = extract_string(args, 0)?;
    let rp = tramp_mod::RemotePath::parse(&path_str)
        .ok_or_else(|| format!("invalid TRAMP path: {}", path_str))?;

    let mut pairs: Vec<(Value, Value)> = vec![
        (Value::keyword("method"), Value::string(&rp.method)),
        (Value::keyword("host"), Value::string(&rp.host)),
        (Value::keyword("path"), Value::string(&rp.path)),
    ];
    if let Some(user) = &rp.user {
        pairs.push((Value::keyword("user"), Value::string(user)));
    }
    if let Some(port) = rp.port {
        pairs.push((Value::keyword("port"), Value::Int(port as i64)));
    }
    Ok(Value::map(pairs))
}

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "tramp-read-file",
        Value::Native(prim_tramp_read_file),
        "Read the remote file at PATH and return its contents as a string.",
    );
    ns.intern_with_doc(
        "tramp-write-file",
        Value::Native(prim_tramp_write_file),
        "Write CONTENT to the remote file at PATH.",
    );
    ns.intern_with_doc(
        "tramp-shell-command",
        Value::Native(prim_tramp_shell_command),
        "Execute COMMAND on the remote host and return output.",
    );
    ns.intern_with_doc(
        "tramp-shell-capture",
        Value::Native(prim_tramp_shell_capture),
        "Execute COMMAND on the remote host and return its stdout.",
    );
    ns.intern_with_doc(
        "tramp-exists?",
        Value::Native(prim_tramp_file_exists),
        "Return t if the remote file at PATH exists.",
    );
    ns.intern_with_doc(
        "tramp-mtime",
        Value::Native(prim_tramp_file_mtime),
        "Return the modification time of the remote file at PATH.",
    );
    ns.intern_with_doc(
        "tramp-list-dir",
        Value::Native(prim_tramp_list_dir),
        "Return a list of file names in the remote directory DIR.",
    );
    ns.intern_with_doc(
        "tramp-mkdir",
        Value::Native(prim_tramp_mkdir),
        "Create the remote directory at PATH.",
    );
    ns.intern_with_doc(
        "tramp-delete-file",
        Value::Native(prim_tramp_delete_file),
        "Delete the remote file at PATH.",
    );
    ns.intern_with_doc(
        "tramp-rename-file",
        Value::Native(prim_tramp_rename_file),
        "Rename the remote file from OLD to NEW.",
    );
    ns.intern_with_doc(
        "tramp-ping",
        Value::Native(prim_tramp_ping),
        "Return t if the remote host is reachable.",
    );
    ns.intern_with_doc(
        "tramp-connect",
        Value::Native(prim_tramp_connect),
        "Establish a connection to the remote host.",
    );
    ns.intern_with_doc(
        "tramp-disconnect",
        Value::Native(prim_tramp_disconnect),
        "Close the connection to the remote host.",
    );
    ns.intern_with_doc(
        "tramp-connections",
        Value::Native(prim_tramp_connections),
        "Return a list of active remote connections.",
    );
    ns.intern_with_doc(
        "tramp-parse-path",
        Value::Native(prim_tramp_parse_path),
        "Parse a tramp PATH and return its components as an alist.",
    );
}
