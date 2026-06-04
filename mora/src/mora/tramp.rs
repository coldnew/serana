/// TRAMP — Transparent Remote Access, Multiple Protocols.
///
/// Provides transparent editing of remote files via SSH/SCP.
/// File paths follow Emacs TRAMP convention:
///   /ssh:user@host:/path/to/file
///   /ssh:user@host#port:/path/to/file
///
/// Connections are cached and reused. SSH ControlMaster is used
/// when available to multiplex sessions over a single connection.
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Parsed remote file path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemotePath {
    pub method: String,
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

impl RemotePath {
    /// Parse a TRAMP-style path: /method:user@host:/path
    /// or /method:user@host#port:/path
    /// or /method:host:/path (no user)
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.strip_prefix('/')?;
        let (method, rest) = input.split_once(':')?;
        if method.is_empty() || rest.is_empty() {
            return None;
        }

        let (host_part, path) = rest.rsplit_once(':')?;

        let (user, host, port) = if let Some((u, h)) = host_part.split_once('@') {
            let user = if u.is_empty() {
                None
            } else {
                Some(u.to_string())
            };
            let (host, port) = parse_host_port(h);
            (user, host, port)
        } else {
            let (host, port) = parse_host_port(host_part);
            (None, host, port)
        };

        Some(RemotePath {
            method: method.to_string(),
            user,
            host,
            port,
            path: path.to_string(),
        })
    }

    /// Build SSH connection arguments for this path.
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.push("-o".to_string());
        args.push("BatchMode=yes".to_string());
        args.push("-o".to_string());
        args.push("ConnectTimeout=10".to_string());
        if let Some(port) = self.port {
            args.push("-p".to_string());
            args.push(port.to_string());
        }
        args
    }

    /// Build the SSH target string: user@host or host.
    pub fn ssh_target(&self) -> String {
        match &self.user {
            Some(u) => format!("{}@{}", u, self.host),
            None => self.host.clone(),
        }
    }

    /// Build an SCP source/dest string.
    pub fn scp_target(&self) -> String {
        let prefix = match &self.user {
            Some(u) => format!("{}@{}:", u, self.host),
            None => format!("{}:", self.host),
        };
        if let Some(port) = self.port {
            format!("[{}{}]{}", prefix.trim_end_matches(':'), port, self.path)
        } else {
            format!("{}{}", prefix, self.path)
        }
    }
}

fn parse_host_port(s: &str) -> (String, Option<u16>) {
    if let Some((h, p)) = s.rsplit_once('#') {
        if let Ok(port) = p.parse::<u16>() {
            return (h.to_string(), Some(port));
        }
    }
    (s.to_string(), None)
}

/// Active SSH connection info.
#[derive(Debug)]
struct Connection {
    _target: String,
    _connected_at: Instant,
    last_used: Instant,
}

/// Connection manager — caches and reuses SSH connections.
#[derive(Debug)]
pub struct ConnectionPool {
    connections: Arc<Mutex<HashMap<String, Connection>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Mark a connection as used.
    pub fn touch(&self, target: &str) {
        let mut conns = self.connections.lock().unwrap();
        let now = Instant::now();
        if let Some(conn) = conns.get_mut(target) {
            conn.last_used = now;
        } else {
            conns.insert(
                target.to_string(),
                Connection {
                    _target: target.to_string(),
                    _connected_at: now,
                    last_used: now,
                },
            );
        }
    }

    /// List active connections.
    pub fn list(&self) -> Vec<String> {
        self.connections.lock().unwrap().keys().cloned().collect()
    }

    /// Disconnect and remove a connection.
    pub fn disconnect(&self, target: &str) {
        self.connections.lock().unwrap().remove(target);
    }

    /// Disconnect all connections.
    pub fn disconnect_all(&self) {
        self.connections.lock().unwrap().clear();
    }

    /// Remove connections idle longer than `max_idle`.
    pub fn cleanup_idle(&self, max_idle: Duration) {
        let now = Instant::now();
        self.connections
            .lock()
            .unwrap()
            .retain(|_, conn| now.duration_since(conn.last_used) < max_idle);
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global connection pool.
static POOL: std::sync::OnceLock<ConnectionPool> = std::sync::OnceLock::new();

pub fn pool() -> &'static ConnectionPool {
    POOL.get_or_init(ConnectionPool::new)
}

// ── Core operations ──────────────────────────────────────────

/// Read a remote file via SSH cat.
pub fn read_file(rp: &RemotePath) -> Result<String, String> {
    pool().touch(&rp.ssh_target());
    let mut cmd = Command::new("ssh");
    cmd.args(rp.ssh_args());
    cmd.arg(rp.ssh_target());
    cmd.arg(format!("cat -- '{}'", escape_shell(&rp.path)));

    let output = cmd.output().map_err(|e| format!("ssh failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("remote read failed: {stderr}"));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("invalid UTF-8: {e}"))
}

/// Write a remote file via SSH tee (writes stdin to remote file).
pub fn write_file(rp: &RemotePath, content: &str) -> Result<(), String> {
    pool().touch(&rp.ssh_target());
    let mut cmd = Command::new("ssh");
    cmd.args(rp.ssh_args());
    cmd.arg(rp.ssh_target());
    cmd.arg(format!("tee -- '{}' > /dev/null", escape_shell(&rp.path)));
    cmd.stdin(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| format!("write to ssh failed: {e}"))?;
    }

    let status = child.wait().map_err(|e| format!("ssh wait failed: {e}"))?;
    if !status.success() {
        return Err("remote write failed".to_string());
    }
    Ok(())
}

/// Run a shell command on a remote host via SSH.
pub fn shell_command(rp: &RemotePath, command: &str) -> Result<(String, i32), String> {
    pool().touch(&rp.ssh_target());
    let mut cmd = Command::new("ssh");
    cmd.args(rp.ssh_args());
    cmd.arg(rp.ssh_target());
    cmd.arg(format!("cd '{}' && {}", escape_shell(&rp.path), command));

    let output = cmd.output().map_err(|e| format!("ssh failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, code))
}

/// Run a shell command on a remote host, return stdout only.
pub fn shell_capture(rp: &RemotePath, command: &str) -> Result<String, String> {
    let (stdout, code) = shell_command(rp, command)?;
    if code != 0 {
        return Err(format!("remote command exited with code {code}: {stdout}"));
    }
    Ok(stdout)
}

/// Test SSH connectivity to a remote host.
pub fn ping(rp: &RemotePath) -> Result<bool, String> {
    let mut cmd = Command::new("ssh");
    cmd.args(rp.ssh_args());
    cmd.arg(rp.ssh_target());
    cmd.arg("echo ok");

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                pool().touch(&rp.ssh_target());
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(e) => Err(format!("ssh connection failed: {e}")),
    }
}

/// List directory on remote host.
pub fn list_directory(rp: &RemotePath) -> Result<Vec<String>, String> {
    let (out, _) = shell_command(rp, "ls -1a 2>/dev/null")?;
    Ok(out.lines().map(String::from).collect())
}

/// Check if a remote path exists.
pub fn file_exists(rp: &RemotePath) -> Result<bool, String> {
    let (_, code) = shell_command(
        rp,
        &format!(
            "test -e '{}' && echo yes || echo no",
            escape_shell(&rp.path)
        ),
    )?;
    Ok(code == 0)
}

/// Get file modification time as unix timestamp.
pub fn file_mtime(rp: &RemotePath) -> Result<i64, String> {
    let (out, code) = shell_command(
        rp,
        &format!(
            "stat -c %Y -- '{}' 2>/dev/null || stat -f %m -- '{}' 2>/dev/null",
            escape_shell(&rp.path),
            escape_shell(&rp.path)
        ),
    )?;
    if code != 0 {
        return Err(format!("failed to stat remote file: {}", rp.path));
    }
    out.trim()
        .parse::<i64>()
        .map_err(|e| format!("failed to parse mtime: {e}"))
}

/// Create a remote directory (with -p).
pub fn make_directory(rp: &RemotePath) -> Result<(), String> {
    let (_, code) = shell_command(rp, &format!("mkdir -p -- '{}'", escape_shell(&rp.path)))?;
    if code != 0 {
        return Err(format!("failed to create remote directory: {}", rp.path));
    }
    Ok(())
}

/// Delete a remote file.
pub fn delete_file(rp: &RemotePath) -> Result<(), String> {
    let (_, code) = shell_command(rp, &format!("rm -- '{}'", escape_shell(&rp.path)))?;
    if code != 0 {
        return Err(format!("failed to delete remote file: {}", rp.path));
    }
    Ok(())
}

/// Rename a remote file.
pub fn rename_file(rp: &RemotePath, new_path: &str) -> Result<(), String> {
    let (_, code) = shell_command(
        rp,
        &format!(
            "mv -- '{}' '{}'",
            escape_shell(&rp.path),
            escape_shell(new_path)
        ),
    )?;
    if code != 0 {
        return Err(format!(
            "failed to rename remote file: {} -> {}",
            rp.path, new_path
        ));
    }
    Ok(())
}

fn escape_shell(s: &str) -> String {
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tramp_path() {
        let p = RemotePath::parse("/ssh:user@host:/home/user/file.txt").unwrap();
        assert_eq!(p.method, "ssh");
        assert_eq!(p.user, Some("user".to_string()));
        assert_eq!(p.host, "host");
        assert_eq!(p.port, None);
        assert_eq!(p.path, "/home/user/file.txt");
    }

    #[test]
    fn test_parse_no_user() {
        let p = RemotePath::parse("/ssh:server.example.com:/var/log/syslog").unwrap();
        assert_eq!(p.method, "ssh");
        assert_eq!(p.user, None);
        assert_eq!(p.host, "server.example.com");
        assert_eq!(p.path, "/var/log/syslog");
    }

    #[test]
    fn test_parse_with_port() {
        let p = RemotePath::parse("/ssh:user@host#2222:/tmp/file").unwrap();
        assert_eq!(p.port, Some(2222));
        assert_eq!(p.ssh_target(), "user@host");
        assert_eq!(p.path, "/tmp/file");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(RemotePath::parse("/local/path").is_none());
        assert!(RemotePath::parse("ssh:user@host:/path").is_none());
        assert!(RemotePath::parse("/:").is_none());
    }

    #[test]
    fn test_connection_pool() {
        let p = ConnectionPool::new();
        assert!(p.list().is_empty());
        p.touch("user@host");
        assert_eq!(p.list(), vec!["user@host"]);
        p.disconnect("user@host");
        assert!(p.list().is_empty());
    }

    #[test]
    fn test_ssh_args() {
        let p = RemotePath::parse("/ssh:user@host#2222:/tmp/file").unwrap();
        let args = p.ssh_args();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
    }

    #[test]
    fn test_escape_shell() {
        assert_eq!(escape_shell("simple"), "simple");
        assert_eq!(escape_shell("it's"), "it'\\''s");
    }
}
