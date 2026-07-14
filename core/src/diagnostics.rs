use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::config::{AppConfig, GatewayConfig, GatewayServerConfig, GatewayTransport};
use crate::{CoreError, Result};

pub const EVENTS_EXCERPT_MAX_LINES: usize = 50;
pub const EVENTS_EXCERPT_MAX_BYTES: usize = 32_768;

pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, child) in map {
                let lowered = key.to_ascii_lowercase();
                if is_secret_key(&lowered) {
                    redacted.insert(key.clone(), Value::String("[redacted]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_value(child));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        other => other.clone(),
    }
}

fn is_secret_key(key: &str) -> bool {
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("api_key")
        || key.contains("credential")
        || key.contains("authorization")
        || key == "value"
        || key == "access_token"
        || key == "refresh_token"
}

pub fn redact_gateway_config(config: &GatewayConfig) -> Value {
    let mut servers = Vec::new();
    for server in &config.servers {
        servers.push(redact_gateway_server(server));
    }
    json!({
        "default_call_timeout_secs": config.default_call_timeout_secs,
        "servers": servers,
    })
}

fn redact_gateway_server(server: &GatewayServerConfig) -> Value {
    let transport = match &server.transport {
        GatewayTransport::Stdio { command, args, env } => json!({
            "type": "stdio",
            "command": command,
            "args": args,
            "env": env.iter().map(|(name, _)| json!({"name": name, "value": "[redacted]"})).collect::<Vec<_>>(),
        }),
        GatewayTransport::StreamableHttp { endpoint, headers } => json!({
            "type": "streamable_http",
            "endpoint": endpoint,
            "headers": headers.iter().map(|(name, _)| json!({"name": name, "value": "[redacted]"})).collect::<Vec<_>>(),
        }),
    };
    let oauth = server.oauth.as_ref().map(|oauth| {
        json!({
            "issuer": oauth.issuer,
            "authorization_endpoint": oauth.authorization_endpoint,
            "token_endpoint": oauth.token_endpoint,
            "client_id": oauth.client_id,
            "scopes": oauth.scopes,
            "token_ref": oauth.token_ref,
        })
    });
    json!({
        "id": server.id,
        "display_name": server.display_name,
        "enabled": server.enabled,
        "scope": server.scope,
        "timeout_secs": server.timeout_secs,
        "credentials": server.credentials.iter().map(|binding| json!({
            "credential_ref": binding.credential_ref,
            "target": binding.target,
        })).collect::<Vec<_>>(),
        "transport": transport,
        "oauth": oauth,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EventsExcerpt {
    conversation_id: String,
    lines: Vec<String>,
}

pub(crate) fn collect_events_excerpts(
    conversations_root: &Path,
    max_lines: usize,
) -> Result<Vec<EventsExcerpt>> {
    let mut excerpts = Vec::new();
    if !conversations_root.is_dir() {
        return Ok(excerpts);
    }
    for entry in fs::read_dir(conversations_root)? {
        let entry = entry?;
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let events_path = folder.join("events.jsonl");
        if !events_path.is_file() {
            continue;
        }
        let meta_path = folder.join("meta.json");
        let conversation_id = if meta_path.is_file() {
            fs::read_to_string(&meta_path)
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|value| {
                    value
                        .get("id")
                        .and_then(|id| id.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| {
                    folder
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                })
        } else {
            folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        let raw = fs::read_to_string(&events_path)?;
        let lines: Vec<String> = raw
            .lines()
            .rev()
            .take(max_lines)
            .map(str::to_string)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .filter_map(|line| redact_event_line(&line).ok())
            .collect();
        if !lines.is_empty() {
            excerpts.push(EventsExcerpt {
                conversation_id,
                lines,
            });
        }
    }
    Ok(excerpts)
}

fn redact_event_line(line: &str) -> Result<String> {
    let mut value: Value = serde_json::from_str(line)?;
    value = redact_value(&value);
    Ok(serde_json::to_string(&value)?)
}

pub fn write_diagnostics_bundle(
    vault_root: &Path,
    dest_path: &Path,
    app_config: &AppConfig,
    harness_health: &Value,
    vault_issues: &Value,
    system_info: &Value,
) -> Result<PathBuf> {
    let dest_path = if dest_path.extension().is_some_and(|ext| ext == "zip") {
        dest_path.to_path_buf()
    } else {
        dest_path.with_extension("zip")
    };
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(&dest_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "vault_path": vault_root.display().to_string(),
        "system": redact_value(system_info),
        "vault_issues": vault_issues,
        "harness_health": harness_health,
        "gateway": redact_gateway_config(&app_config.gateway),
        "agent_roster": app_config.agent_roster.iter().map(|agent| json!({
            "id": agent.id,
            "display_name": agent.display_name,
            "command": agent.command,
            "args": agent.args,
        })).collect::<Vec<_>>(),
    });
    write_zip_json(&mut zip, "manifest.json", &manifest, options)?;

    let excerpts =
        collect_events_excerpts(&vault_root.join("conversations"), EVENTS_EXCERPT_MAX_LINES)?;
    let mut events_bytes = 0usize;
    let mut capped_excerpts = Vec::new();
    for excerpt in excerpts {
        let payload = serde_json::to_string(&excerpt)?;
        if events_bytes.saturating_add(payload.len()) > EVENTS_EXCERPT_MAX_BYTES {
            break;
        }
        events_bytes = events_bytes.saturating_add(payload.len());
        capped_excerpts.push(excerpt);
    }
    write_zip_json(
        &mut zip,
        "events_excerpts.json",
        &json!(capped_excerpts),
        options,
    )?;

    let readme = diagnostics_readme(vault_root, &dest_path);
    zip.start_file("README.txt", options)
        .map_err(|err| CoreError::Protocol(format!("diagnostics zip: {err}")))?;
    zip.write_all(readme.as_bytes())
        .map_err(|err| CoreError::Protocol(format!("diagnostics zip: {err}")))?;

    zip.finish()
        .map_err(|err| CoreError::Protocol(format!("diagnostics zip: {err}")))?;
    Ok(dest_path)
}

fn write_zip_json<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    value: &Value,
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)
        .map_err(|err| CoreError::Protocol(format!("diagnostics zip: {err}")))?;
    let bytes = serde_json::to_vec_pretty(value)?;
    zip.write_all(&bytes)
        .map_err(|err| CoreError::Protocol(format!("diagnostics zip: {err}")))?;
    Ok(())
}

fn diagnostics_readme(vault_root: &Path, bundle_path: &Path) -> String {
    format!(
        "tamtri diagnostics bundle\n\
         \n\
         Generated for user review before sharing. Nothing uploads automatically.\n\
         \n\
         Vault: {}\n\
         Bundle: {}\n\
         \n\
         Contents:\n\
         - manifest.json: app/system/harness/gateway metadata (secrets redacted)\n\
         - events_excerpts.json: recent audit log lines (capped, redacted)\n\
         \n\
         Attach this zip to a GitHub issue only after reviewing for sensitive paths or data.\n",
        vault_root.display(),
        bundle_path.display()
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::config::GatewayConfig;
    use crate::conversation::Conversation;
    use crate::vault::ConversationVault;
    use crate::vault::fs::FilesystemVault;
    use std::fs;

    #[test]
    fn diagnostics_bundle_redacts_secrets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault = FilesystemVault::new(temp.path()).expect("vault");
        let conversation = Conversation::new("Diag");
        vault.create(&conversation).expect("create");
        let folder = temp.path().join("conversations");
        let conv_dir = fs::read_dir(&folder)
            .expect("read")
            .find_map(|entry| entry.ok())
            .expect("folder")
            .path();
        let events_path = conv_dir.join("events.jsonl");
        fs::write(
            &events_path,
            r#"{"ts":"2026-07-03T12:00:00Z","kind":"gateway_credential_injected","payload":{"credential_ref":"api_key","access_token":"super-secret"}}"#,
        )
        .expect("events");

        let config = AppConfig {
            gateway: GatewayConfig {
                default_call_timeout_secs: 300,
                servers: vec![GatewayServerConfig {
                    id: "remote".into(),
                    display_name: "Remote".into(),
                    enabled: true,
                    scope: crate::config::GatewayScope::User,
                    transport: GatewayTransport::Stdio {
                        command: "mcp".into(),
                        args: vec![],
                        env: vec![("API_KEY".into(), "secret-value".into())],
                    },
                    timeout_secs: None,
                    credentials: vec![crate::config::CredentialBinding {
                        credential_ref: "api_key".into(),
                        target: crate::config::CredentialTarget::EnvVar {
                            name: "API_KEY".into(),
                        },
                    }],
                    oauth: None,
                }],
            },
            ..Default::default()
        };

        let dest = temp.path().join("diag.zip");
        let bundle = write_diagnostics_bundle(
            temp.path(),
            &dest,
            &config,
            &json!([]),
            &json!([]),
            &json!({"macos_version": "15.0", "api_key": "should-redact"}),
        )
        .expect("bundle");
        assert!(bundle.exists());

        let reader = fs::File::open(&bundle).expect("open");
        let mut archive = zip::ZipArchive::new(reader).expect("archive");
        let mut manifest_raw = String::new();
        archive
            .by_name("manifest.json")
            .expect("manifest")
            .read_to_string(&mut manifest_raw)
            .expect("read");
        assert!(!manifest_raw.contains("secret-value"));
        assert!(!manifest_raw.contains("super-secret"));
        assert!(manifest_raw.contains("[redacted]"));

        let mut events_raw = String::new();
        archive
            .by_name("events_excerpts.json")
            .expect("events")
            .read_to_string(&mut events_raw)
            .expect("read");
        assert!(!events_raw.contains("super-secret"));
        assert!(events_raw.contains("[redacted]"));
    }

    #[test]
    fn vault_duplicate_issue_badge_maps_detail() {
        let issues = json!([{
            "kind": "duplicate_id",
            "conversation_id": "018e1234-5678-7890-abcd-ef0123456789",
            "detail": "Duplicate conversation id"
        }]);
        assert_eq!(issues[0]["kind"], "duplicate_id");
        assert!(
            issues[0]["detail"]
                .as_str()
                .unwrap()
                .contains("Duplicate conversation id")
        );
    }
}
