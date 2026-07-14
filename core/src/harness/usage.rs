//! Provider quota probes for harness agents. Mirrors paseo's quota-fetcher surface
//! at a minimal depth: Codex and Claude file-based credentials only.

use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Result;
use crate::error::CoreError;
use crate::harness::auth::{
    ClaudeCredentials, CLAUDE_OAUTH_CLIENT_ID, read_claude_credentials, save_claude_oauth_tokens,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageWindow {
    pub id: String,
    pub label: String,
    pub utilization_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    pub tone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageBalance {
    pub id: String,
    pub label: String,
    pub remaining: f64,
    pub unit: String,
    pub tone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessUsageEntry {
    pub provider_id: String,
    pub display_name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_label: Option<String>,
    #[serde(default)]
    pub windows: Vec<HarnessUsageWindow>,
    #[serde(default)]
    pub balances: Vec<HarnessUsageBalance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub fetched_at: String,
}

const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

pub fn list_harness_usage(roster_ids: &[String]) -> Vec<HarnessUsageEntry> {
    let now = iso_now();
    let mut entries = Vec::new();
    if roster_ids.iter().any(|id| id.starts_with("codex")) {
        entries.push(fetch_codex_usage(&now));
    }
    if roster_ids
        .iter()
        .any(|id| id.starts_with("claude") || id.contains("claude"))
    {
        entries.push(fetch_claude_usage(&now));
    }
    entries
}

fn fetch_codex_usage(fetched_at: &str) -> HarnessUsageEntry {
    let base = HarnessUsageEntry {
        provider_id: "codex".into(),
        display_name: "Codex".into(),
        status: "unavailable".into(),
        plan_label: None,
        windows: Vec::new(),
        balances: Vec::new(),
        error: None,
        fetched_at: fetched_at.to_string(),
    };
    let Some(auth) = read_codex_auth() else {
        return base;
    };
    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return HarnessUsageEntry {
                error: Some(err.to_string()),
                ..base
            };
        }
    };
    match codex_usage_from_auth(&client, &auth) {
        Ok(entry) => entry,
        Err(err) => HarnessUsageEntry {
            error: Some(err.to_string()),
            ..base
        },
    }
}

fn fetch_claude_usage(fetched_at: &str) -> HarnessUsageEntry {
    let base = HarnessUsageEntry {
        provider_id: "claude".into(),
        display_name: "Claude".into(),
        status: "unavailable".into(),
        plan_label: None,
        windows: Vec::new(),
        balances: Vec::new(),
        error: None,
        fetched_at: fetched_at.to_string(),
    };
    let Some(creds) = read_claude_credentials() else {
        return base;
    };
    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return HarnessUsageEntry {
                error: Some(err.to_string()),
                ..base
            };
        }
    };
    match claude_usage_from_credentials(&client, &creds) {
        Ok(entry) => entry,
        Err(err) => HarnessUsageEntry {
            error: Some(err.to_string()),
            ..base
        },
    }
}

struct CodexAuth {
    access_token: String,
    refresh_token: Option<String>,
    account_id: Option<String>,
    path: PathBuf,
    raw: Value,
}

fn codex_usage_from_auth(client: &Client, auth: &CodexAuth) -> Result<HarnessUsageEntry> {
    let mut token = auth.access_token.clone();
    let mut resp = call_codex_api(client, &token, auth.account_id.as_deref())?;
    if matches!(resp, CodexApiResult::NeedsAuth) {
        let Some(refresh) = auth.refresh_token.as_deref() else {
            return Ok(unavailable_codex());
        };
        let refreshed = refresh_codex_token(client, refresh)?;
        let Some(new_token) = refreshed.access_token else {
            return Ok(unavailable_codex());
        };
        if let Some(refresh_token) = refreshed.refresh_token {
            save_codex_tokens(&auth.path, &auth.raw, &new_token, &refresh_token)?;
        }
        token = new_token;
        resp = call_codex_api(client, &token, auth.account_id.as_deref())?;
        if matches!(resp, CodexApiResult::NeedsAuth) {
            return Ok(unavailable_codex());
        }
    }
    let CodexApiResult::Ok(body) = resp else {
        return Ok(unavailable_codex());
    };
    Ok(codex_body_to_usage(&body))
}

fn unavailable_codex() -> HarnessUsageEntry {
    HarnessUsageEntry {
        provider_id: "codex".into(),
        display_name: "Codex".into(),
        status: "unavailable".into(),
        plan_label: None,
        windows: Vec::new(),
        balances: Vec::new(),
        error: None,
        fetched_at: iso_now(),
    }
}

fn codex_body_to_usage(body: &Value) -> HarnessUsageEntry {
    let mut windows = Vec::new();
    if let Some(primary) = body.pointer("/rate_limit/primary_window")
        && let Some(window) = codex_window_from_json(primary, "session", "Session", "ok")
    {
        windows.push(window);
    }
    if let Some(secondary) = body.pointer("/rate_limit/secondary_window")
        && let Some(window) = codex_window_from_json(secondary, "weekly", "Weekly", "warning")
    {
        windows.push(window);
    }
    let mut balances = Vec::new();
    if let Some(balance) = body.pointer("/credits/balance").and_then(Value::as_f64) {
        balances.push(HarnessUsageBalance {
            id: "credits".into(),
            label: "Credits".into(),
            remaining: balance,
            unit: "usd".into(),
            tone: balance_tone(balance),
        });
    }
    HarnessUsageEntry {
        provider_id: "codex".into(),
        display_name: "Codex".into(),
        status: if windows.is_empty() && balances.is_empty() {
            "unavailable".into()
        } else {
            "available".into()
        },
        plan_label: body
            .get("plan_type")
            .and_then(Value::as_str)
            .map(str::to_string),
        windows,
        balances,
        error: None,
        fetched_at: iso_now(),
    }
}

fn codex_window_from_json(
    value: &Value,
    id: &str,
    label: &str,
    default_tone: &str,
) -> Option<HarnessUsageWindow> {
    let used = value.get("used_percent")?.as_f64()?;
    let tone = if used >= 70.0 {
        "warning"
    } else {
        default_tone
    };
    let resets_at = value
        .get("reset_at")
        .and_then(Value::as_f64)
        .map(format_reset_epoch);
    Some(HarnessUsageWindow {
        id: id.to_string(),
        label: label.to_string(),
        utilization_pct: used,
        resets_at,
        tone: tone.to_string(),
    })
}

enum CodexApiResult {
    Ok(Value),
    NeedsAuth,
}

#[derive(Debug, Deserialize)]
struct CodexTokenRefresh {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

fn reqwest_err(err: reqwest::Error) -> CoreError {
    CoreError::Protocol(err.to_string())
}

fn call_codex_api(
    client: &Client,
    token: &str,
    account_id: Option<&str>,
) -> Result<CodexApiResult> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {token}").parse().expect("header"),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/json".parse().expect("header"),
    );
    if let Some(id) = account_id {
        headers.insert("ChatGPT-Account-Id", id.parse().expect("header"));
    }
    let resp = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .headers(headers)
        .send()
        .map_err(reqwest_err)?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED
        || resp.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Ok(CodexApiResult::NeedsAuth);
    }
    if !resp.status().is_success() {
        return Err(crate::CoreError::Protocol(format!(
            "Codex usage API returned {}",
            resp.status()
        )));
    }
    let text = resp.text().map_err(reqwest_err)?;
    if text.trim_start().starts_with('<') {
        return Ok(CodexApiResult::NeedsAuth);
    }
    Ok(CodexApiResult::Ok(serde_json::from_str(&text)?))
}

fn refresh_codex_token(client: &Client, refresh_token: &str) -> Result<CodexTokenRefresh> {
    let resp = client
        .post("https://auth.openai.com/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&client_id={CODEX_CLIENT_ID}&refresh_token={refresh_token}"
        ))
        .send()
        .map_err(reqwest_err)?;
    if !resp.status().is_success() {
        return Err(crate::CoreError::Protocol(format!(
            "Codex token refresh returned {}",
            resp.status()
        )));
    }
    resp.json().map_err(reqwest_err)
}

fn save_codex_tokens(path: &Path, raw: &Value, access: &str, refresh: &str) -> Result<()> {
    let mut updated = raw.clone();
    if let Some(tokens) = updated
        .pointer_mut("/tokens")
        .and_then(Value::as_object_mut)
    {
        tokens.insert("access_token".into(), Value::String(access.to_string()));
        tokens.insert("refresh_token".into(), Value::String(refresh.to_string()));
    }
    let bytes = serde_json::to_vec_pretty(&updated)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn read_codex_auth() -> Option<CodexAuth> {
    let home = dirs::home_dir()?;
    let mut candidates = Vec::new();
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        candidates.push(PathBuf::from(codex_home).join("auth.json"));
    }
    candidates.push(home.join(".config/codex/auth.json"));
    candidates.push(home.join(".codex/auth.json"));
    candidates.into_iter().find_map(|path| {
        let raw: Value = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
        let access_token = raw
            .pointer("/tokens/access_token")
            .and_then(Value::as_str)?
            .to_string();
        Some(CodexAuth {
            refresh_token: raw
                .pointer("/tokens/refresh_token")
                .and_then(Value::as_str)
                .map(str::to_string),
            account_id: raw
                .pointer("/tokens/account_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            access_token,
            path,
            raw,
        })
    })
}

enum ClaudeApiResult {
    Ok(Value),
    NeedsAuth,
}

#[derive(Debug, Deserialize)]
struct ClaudeTokenRefresh {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

fn claude_usage_from_credentials(client: &Client, creds: &ClaudeCredentials) -> Result<HarnessUsageEntry> {
    let plan_label = claude_plan_label(
        creds.subscription_type.as_deref(),
        creds.rate_limit_tier.as_deref(),
    );
    let mut token = creds.access_token.clone();
    let mut resp = call_claude_api(client, &token)?;
    if matches!(resp, ClaudeApiResult::NeedsAuth) {
        let Some(refresh) = creds.refresh_token.as_deref() else {
            return Ok(unavailable_claude_with_error(
                plan_label,
                "Claude authentication expired. Sign in again with `claude login`.",
            ));
        };
        let refreshed = refresh_claude_token(client, refresh)?;
        let Some(new_token) = refreshed.access_token else {
            return Ok(unavailable_claude_with_error(
                plan_label,
                "Claude token refresh failed. Sign in again with `claude login`.",
            ));
        };
        let new_refresh = refreshed
            .refresh_token
            .as_deref()
            .unwrap_or(refresh);
        save_claude_oauth_tokens(creds, &new_token, new_refresh)?;
        token = new_token;
        resp = call_claude_api(client, &token)?;
        if matches!(resp, ClaudeApiResult::NeedsAuth) {
            return Ok(unavailable_claude_with_error(
                plan_label,
                "Claude authentication expired. Sign in again with `claude login`.",
            ));
        }
    }
    let ClaudeApiResult::Ok(body) = resp else {
        return Ok(unavailable_claude());
    };
    Ok(claude_body_to_usage(&body, plan_label))
}

fn unavailable_claude() -> HarnessUsageEntry {
    HarnessUsageEntry {
        provider_id: "claude".into(),
        display_name: "Claude".into(),
        status: "unavailable".into(),
        plan_label: None,
        windows: Vec::new(),
        balances: Vec::new(),
        error: None,
        fetched_at: iso_now(),
    }
}

fn unavailable_claude_with_error(plan_label: Option<String>, error: &str) -> HarnessUsageEntry {
    HarnessUsageEntry {
        error: Some(error.to_string()),
        plan_label,
        ..unavailable_claude()
    }
}

fn claude_body_to_usage(body: &Value, plan_label: Option<String>) -> HarnessUsageEntry {
    let mut windows = Vec::new();
    if let Some(five_hour) = body.get("five_hour")
        && let Some(window) = claude_window_from_json(five_hour, "session", "Session")
    {
        windows.push(window);
    }
    if let Some(seven_day) = body.get("seven_day")
        && let Some(window) = claude_window_from_json(seven_day, "weekly", "Weekly")
    {
        windows.push(window);
    }
    if let Some(seven_day_opus) = body.get("seven_day_opus")
        && let Some(window) = claude_window_from_json(seven_day_opus, "weekly_opus", "Weekly · Opus")
    {
        windows.push(window);
    }
    if let Some(seven_day_omelette) = body.get("seven_day_omelette")
        && let Some(window) =
            claude_window_from_json(seven_day_omelette, "weekly_omelette", "Weekly · Omelette")
    {
        windows.push(window);
    }
    HarnessUsageEntry {
        provider_id: "claude".into(),
        display_name: "Claude".into(),
        status: if windows.is_empty() {
            "unavailable".into()
        } else {
            "available".into()
        },
        plan_label,
        windows,
        balances: Vec::new(),
        error: None,
        fetched_at: iso_now(),
    }
}

fn call_claude_api(client: &Client, token: &str) -> Result<ClaudeApiResult> {
    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .send()
        .map_err(reqwest_err)?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED
        || resp.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Ok(ClaudeApiResult::NeedsAuth);
    }
    if !resp.status().is_success() {
        return Err(crate::CoreError::Protocol(format!(
            "Claude usage API returned {}",
            resp.status()
        )));
    }
    Ok(ClaudeApiResult::Ok(resp.json().map_err(reqwest_err)?))
}

fn refresh_claude_token(client: &Client, refresh_token: &str) -> Result<ClaudeTokenRefresh> {
    let resp = client
        .post("https://platform.claude.com/v1/oauth/token")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": CLAUDE_OAUTH_CLIENT_ID,
            "scope": "user:profile user:inference user:sessions:claude_code user:mcp_servers",
        }))
        .send()
        .map_err(reqwest_err)?;
    if !resp.status().is_success() {
        return Err(crate::CoreError::Protocol(format!(
            "Claude token refresh returned {}",
            resp.status()
        )));
    }
    resp.json().map_err(reqwest_err)
}

fn claude_plan_label(subscription_type: Option<&str>, rate_limit_tier: Option<&str>) -> Option<String> {
    let subscription_type = subscription_type?;
    let label = {
        let mut chars = subscription_type.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => return None,
        }
    };
    let tier = rate_limit_tier
        .and_then(|tier| tier.split('_').next_back())
        .filter(|tier| !tier.is_empty());
    Some(match tier {
        Some(tier) => format!("{label} {tier}"),
        None => label,
    })
}

fn claude_window_from_json(value: &Value, id: &str, label: &str) -> Option<HarnessUsageWindow> {
    let used = value.get("utilization")?.as_f64()?;
    let resets_at = value
        .get("resets_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    let tone = if used >= 70.0 { "warning" } else { "ok" };
    Some(HarnessUsageWindow {
        id: id.to_string(),
        label: label.to_string(),
        utilization_pct: used,
        resets_at,
        tone: tone.to_string(),
    })
}

/// Whether Claude Code credentials exist locally (readiness probe, not quota).
pub fn claude_credentials_present() -> bool {
    crate::harness::auth::claude_credentials_present()
}

/// Whether Codex credentials exist locally (readiness probe, not quota).
pub fn codex_credentials_present() -> bool {
    crate::harness::auth::codex_credentials_present()
}

fn balance_tone(remaining: f64) -> String {
    if remaining <= 0.0 {
        "danger".into()
    } else if remaining < 5.0 {
        "warning".into()
    } else {
        "ok".into()
    }
}

fn format_reset_epoch(secs: f64) -> String {
    let millis = (secs * 1000.0) as u64;
    chrono::DateTime::from_timestamp_millis(millis as i64)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| secs.to_string())
}

fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_harness_usage_empty_roster() {
        assert!(list_harness_usage(&[]).is_empty());
    }

    #[test]
    fn list_harness_usage_includes_codex_for_codex_native() {
        let ids = vec!["codex-native".to_string()];
        let entries = list_harness_usage(&ids);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].provider_id, "codex");
    }

    #[test]
    fn claude_plan_label_formats_subscription_and_tier() {
        assert_eq!(
            claude_plan_label(Some("max"), Some("default_max")),
            Some("Max max".into())
        );
        assert_eq!(claude_plan_label(Some("pro"), None), Some("Pro".into()));
        assert_eq!(claude_plan_label(None, Some("default_max")), None);
    }

    #[test]
    fn claude_window_from_json_parses_utilization() {
        let value = serde_json::json!({"utilization": 42.5, "resets_at": "2026-07-14T00:00:00Z"});
        let window = claude_window_from_json(&value, "session", "Session").expect("window");
        assert_eq!(window.utilization_pct, 42.5);
        assert_eq!(window.resets_at.as_deref(), Some("2026-07-14T00:00:00Z"));
        assert_eq!(window.tone, "ok");
    }
}
