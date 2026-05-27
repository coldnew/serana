use regex::Regex;
use std::sync::LazyLock;

/// Patterns that match common secret/token formats.
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // API keys and tokens (generic)
        Regex::new(r#"(?i)(api[_-]?key|apikey|secret[_-]?key|token|auth|password|passwd|pwd)\s*[:=]\s*["']?([A-Za-z0-9_\-\.]{20,})["']?"#).unwrap(),
        // GitHub tokens
        Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
        // AWS access keys
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        // OpenAI API keys
        Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap(),
        // Anthropic API keys
        Regex::new(r"sk-ant-[A-Za-z0-9\-]{20,}").unwrap(),
        // Generic bearer tokens
        Regex::new(r"(?i)bearer\s+[A-Za-z0-9_\-\.]{20,}").unwrap(),
        // Private keys
        Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
        // Base64-encoded secrets in env vars
        Regex::new(r"(?i)(SECRET|TOKEN|KEY|PASSWORD)\s*=\s*[A-Za-z0-9+/]{40,}={0,2}").unwrap(),
        // JWT tokens
        Regex::new(r"eyJ[A-Za-z0-9_\-]*\.eyJ[A-Za-z0-9_\-]*\.[A-Za-z0-9_\-]+").unwrap(),
        // Slack tokens
        Regex::new(r"xox[bporas]-[A-Za-z0-9\-]+").unwrap(),
        // Generic hex secrets (32+ chars)
        Regex::new(r#"(?i)(secret|token|key|hash)\s*[:=]\s*["']?([a-f0-9]{32,})["']?"#).unwrap(),
    ]
});

/// Redact secrets from text, replacing them with `[REDACTED]`.
pub fn redact_secrets(text: &str) -> String {
    let mut result = text.to_string();
    for pattern in SECRET_PATTERNS.iter() {
        result = pattern
            .replace_all(&result, "[REDACTED]")
            .into_owned();
    }
    result
}

/// Check if a file path looks like it might contain secrets.
pub fn is_secret_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".env")
        || lower.ends_with(".env.local")
        || lower.ends_with(".env.production")
        || lower.contains(".env.")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.contains("secret")
        || lower.contains("credential")
        || lower.contains("token")
        || lower.ends_with(".p12")
        || lower.ends_with(".pfx")
        || lower.contains("id_rsa")
        || lower.contains("id_ed25519")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_key() {
        let text = "Using key sk-abc123def456ghi789jkl012mno345pqr678stu901vwx234";
        let result = redact_secrets(text);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("sk-abc"));
    }

    #[test]
    fn redacts_github_token() {
        let text = "token: ghp_abcdefghijklmnopqrstuvwxyz1234567890";
        let result = redact_secrets(text);
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_private_key_header() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowI...";
        let result = redact_secrets(text);
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn leaves_normal_text_alone() {
        let text = "fn main() { println!(\"hello\"); }";
        let result = redact_secrets(text);
        assert_eq!(result, text);
    }

    #[test]
    fn detects_secret_files() {
        assert!(is_secret_file(".env"));
        assert!(is_secret_file("config.env.local"));
        assert!(is_secret_file("server.pem"));
        assert!(is_secret_file("id_rsa"));
        assert!(!is_secret_file("main.rs"));
        assert!(!is_secret_file("config.toml"));
    }
}
