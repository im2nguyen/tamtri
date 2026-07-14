//! Shared credential probes for native harness adapters (readiness + usage).

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

pub const CLAUDE_KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
pub const CLAUDE_OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
    /// Set when credentials were read from `~/.claude/.credentials.json` (or `CLAUDE_HOME`).
    pub credentials_path: Option<PathBuf>,
    raw: Value,
}

/// Returns Claude Code OAuth credentials from disk or macOS Keychain.
pub fn read_claude_credentials() -> Option<ClaudeCredentials> {
    if let Some((path, raw)) = read_claude_credentials_file()
        && let Some(oauth) = parse_claude_oauth(&raw)
    {
        return Some(ClaudeCredentials {
            access_token: oauth.access_token,
            refresh_token: oauth.refresh_token,
            subscription_type: oauth.subscription_type,
            rate_limit_tier: oauth.rate_limit_tier,
            credentials_path: Some(path),
            raw,
        });
    }

    #[cfg(target_os = "macos")]
    {
        let raw = read_claude_keychain_credentials()?;
        let oauth = parse_claude_oauth(&raw)?;
        Some(ClaudeCredentials {
            access_token: oauth.access_token,
            refresh_token: oauth.refresh_token,
            subscription_type: oauth.subscription_type,
            rate_limit_tier: oauth.rate_limit_tier,
            credentials_path: None,
            raw,
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Returns the Claude Code OAuth access token when present on disk or Keychain.
pub fn read_claude_access_token() -> Option<String> {
    read_claude_credentials().map(|creds| creds.access_token)
}

/// Whether Claude Code is authenticated for local runs.
pub fn claude_auth_ready(command: &str) -> bool {
    if read_claude_credentials().is_some() {
        return true;
    }
    claude_auth_status_logged_in(command)
}

/// Whether Claude Code credentials exist locally (readiness probe, not quota).
pub fn claude_credentials_present() -> bool {
    read_claude_credentials().is_some()
}

/// Persist refreshed OAuth tokens to the on-disk credentials file when available.
pub fn save_claude_oauth_tokens(creds: &ClaudeCredentials, access: &str, refresh: &str) -> crate::Result<()> {
    let Some(path) = creds.credentials_path.as_deref() else {
        return Ok(());
    };
    let mut updated = creds.raw.clone();
    let Some(oauth) = updated
        .get_mut("claudeAiOauth")
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    oauth.insert("accessToken".into(), Value::String(access.to_string()));
    oauth.insert("refreshToken".into(), Value::String(refresh.to_string()));
    let bytes = serde_json::to_vec_pretty(&updated)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

struct ClaudeOAuthFields {
    access_token: String,
    refresh_token: Option<String>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

fn claude_home_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(
        std::env::var("CLAUDE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".claude")),
    )
}

fn claude_credentials_path() -> Option<PathBuf> {
    Some(claude_home_dir()?.join(".credentials.json"))
}

fn read_claude_credentials_file() -> Option<(PathBuf, Value)> {
    let path = claude_credentials_path()?;
    let raw: Value = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
    Some((path, raw))
}

fn parse_claude_oauth(raw: &Value) -> Option<ClaudeOAuthFields> {
    let oauth = raw.get("claudeAiOauth")?;
    let access_token = oauth
        .get("accessToken")
        .and_then(Value::as_str)?
        .to_string();
    Some(ClaudeOAuthFields {
        access_token,
        refresh_token: oauth
            .get("refreshToken")
            .and_then(Value::as_str)
            .map(str::to_string),
        subscription_type: oauth
            .get("subscriptionType")
            .and_then(Value::as_str)
            .map(str::to_string),
        rate_limit_tier: oauth
            .get("rateLimitTier")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

#[cfg(target_os = "macos")]
fn read_claude_keychain_credentials() -> Option<Value> {
    let mut child = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            CLAUDE_KEYCHAIN_SERVICE,
            "-w",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            if !status.success() {
                return None;
            }
            let mut stdout = Vec::new();
            child
                .stdout
                .take()?
                .read_to_end(&mut stdout)
                .ok()?;
            let raw = String::from_utf8(stdout).ok()?.trim().to_string();
            if raw.is_empty() {
                return None;
            }
            return serde_json::from_str(&raw).ok();
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Whether Codex credentials exist locally (readiness probe, not quota).
pub fn codex_credentials_present() -> bool {
    codex_auth_paths().into_iter().any(|path| {
        let raw: Value = match std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            Some(raw) => raw,
            None => return false,
        };
        raw.pointer("/tokens/access_token")
            .and_then(Value::as_str)
            .is_some()
    })
}

fn claude_auth_status_logged_in(command: &str) -> bool {
    let mut child = match Command::new(command)
        .args(["auth", "status"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            if !status.success() {
                return false;
            }
            let mut stdout = Vec::new();
            if let Some(mut out) = child.stdout.take()
                && out.read_to_end(&mut stdout).is_ok()
            {
                return parse_claude_auth_logged_in(&stdout);
            }
            return false;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn parse_claude_auth_logged_in(bytes: &[u8]) -> bool {
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    value
        .get("loggedIn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn codex_auth_paths() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        candidates.push(PathBuf::from(codex_home).join("auth.json"));
    }
    candidates.push(home.join(".config/codex/auth.json"));
    candidates.push(home.join(".codex/auth.json"));
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn missing_credentials_do_not_panic() {
        let _ = claude_auth_ready("claude");
        let _ = codex_credentials_present();
    }

    #[test]
    fn parse_claude_auth_status_json() {
        assert!(parse_claude_auth_logged_in(
            br#"{"loggedIn":true,"authMethod":"claude.ai"}"#
        ));
        assert!(!parse_claude_auth_logged_in(br#"{"loggedIn":false}"#));
        assert!(!parse_claude_auth_logged_in(b"not json"));
    }

    #[test]
    fn parse_claude_oauth_from_credentials_json() {
        let raw: Value = serde_json::from_str(
            r#"{"claudeAiOauth":{"accessToken":"tok","refreshToken":"ref","subscriptionType":"max","rateLimitTier":"default_max"}}"#,
        )
        .expect("json");
        let oauth = parse_claude_oauth(&raw).expect("oauth");
        assert_eq!(oauth.access_token, "tok");
        assert_eq!(oauth.refresh_token.as_deref(), Some("ref"));
        assert_eq!(oauth.subscription_type.as_deref(), Some("max"));
        assert_eq!(oauth.rate_limit_tier.as_deref(), Some("default_max"));
    }

    #[test]
    fn read_claude_credentials_from_file() {
        let dir = TempDir::new().expect("tempdir");
        let cred_path = dir.path().join(".credentials.json");
        fs::write(
            &cred_path,
            r#"{"claudeAiOauth":{"accessToken":"file-token","refreshToken":"file-refresh"}}"#,
        )
        .expect("write");
        let raw: Value = serde_json::from_slice(&fs::read(&cred_path).expect("read")).expect("json");
        let oauth = parse_claude_oauth(&raw).expect("oauth");
        assert_eq!(oauth.access_token, "file-token");
    }

    #[test]
    fn save_claude_oauth_tokens_updates_file() {
        let dir = TempDir::new().expect("tempdir");
        let cred_path = dir.path().join(".credentials.json");
        let raw: Value = serde_json::from_str(
            r#"{"claudeAiOauth":{"accessToken":"old","refreshToken":"old-ref"}}"#,
        )
        .expect("json");
        fs::write(&cred_path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");
        let creds = ClaudeCredentials {
            access_token: "old".into(),
            refresh_token: Some("old-ref".into()),
            subscription_type: None,
            rate_limit_tier: None,
            credentials_path: Some(cred_path.clone()),
            raw,
        };
        save_claude_oauth_tokens(&creds, "new-access", "new-refresh").expect("save");
        let updated: Value =
            serde_json::from_slice(&fs::read(&cred_path).expect("read")).expect("json");
        assert_eq!(
            updated.pointer("/claudeAiOauth/accessToken").and_then(Value::as_str),
            Some("new-access")
        );
        assert_eq!(
            updated.pointer("/claudeAiOauth/refreshToken").and_then(Value::as_str),
            Some("new-refresh")
        );
    }
}
