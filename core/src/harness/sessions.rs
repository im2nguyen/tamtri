//! Discover and import native harness session files into the vault.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::conversation::{Conversation, NativeSessionLink};
use crate::harness::claude::parse_claude_session_file;
use crate::protocol::WireU64;
use crate::{CoreError, Result};

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSessionSummary {
    pub provider: String,
    pub path: String,
    pub title: String,
    pub updated_at: WireU64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

const LIST_NATIVE_LIMIT: usize = 50;

pub fn list_native_sessions() -> Vec<NativeSessionSummary> {
    let mut out = Vec::new();
    if let Some(home) = dirs::home_dir() {
        out.extend(scan_claude_jsonl_sessions(
            &home.join(".claude").join("projects"),
            LIST_NATIVE_LIMIT,
        ));
        out.extend(scan_codex_session_dirs(
            &home.join(".codex").join("sessions"),
        ));
    }
    out.sort_by_key(|row| std::cmp::Reverse(row.updated_at));
    out.truncate(LIST_NATIVE_LIMIT);
    out
}

pub fn import_native_session(
    provider: &str,
    path: &str,
    harness_id: &str,
    model_id: &str,
) -> Result<Conversation> {
    let path = PathBuf::from(path);
    match provider {
        "claude" => import_claude_session(&path, harness_id, model_id),
        other => Err(CoreError::Protocol(format!(
            "native session import not supported for provider '{other}'"
        ))),
    }
}

fn import_claude_session(path: &Path, harness_id: &str, model_id: &str) -> Result<Conversation> {
    let parsed = parse_claude_session_file(path)?;
    let title = parsed
        .title
        .clone()
        .unwrap_or_else(|| format!("Claude session {}", &parsed.session_id[..8.min(parsed.session_id.len())]));
    let mut conversation = Conversation::new(title);
    conversation.active_harness_id = Some(harness_id.to_string());
    conversation.model_id = Some(model_id.to_string());
    conversation.messages = parsed.messages;
    conversation.native_session = Some(NativeSessionLink {
        provider: "claude".to_string(),
        session_id: parsed.session_id,
        cwd: parsed.cwd,
        source_path: Some(path.to_string_lossy().to_string()),
    });
    Ok(conversation)
}

fn scan_claude_jsonl_sessions(root: &Path, limit: usize) -> Vec<NativeSessionSummary> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    let Ok(project_dirs) = fs::read_dir(root) else {
        return Vec::new();
    };
    for project_dir in project_dirs.filter_map(|entry| entry.ok()) {
        let project_path = project_dir.path();
        if !project_path.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(&project_path) else {
            continue;
        };
        for file in files.filter_map(|entry| entry.ok()) {
            let path = file.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            let updated_at = modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            candidates.push((path, updated_at));
        }
    }
    candidates.sort_by_key(|(_, updated_at)| std::cmp::Reverse(*updated_at));
    candidates
        .into_iter()
        .take(limit)
        .filter_map(|(path, updated_at)| summarize_claude_jsonl(&path, updated_at))
        .collect()
}

fn summarize_claude_jsonl(path: &Path, updated_at: u64) -> Option<NativeSessionSummary> {
    let parsed = parse_claude_session_file(path).ok()?;
    let title = parsed.title.unwrap_or_else(|| {
        format!(
            "Claude session {}",
            &parsed.session_id[..8.min(parsed.session_id.len())]
        )
    });
    Some(NativeSessionSummary {
        provider: "claude".to_string(),
        path: path.to_string_lossy().to_string(),
        title,
        updated_at,
        session_id: Some(parsed.session_id),
        cwd: Some(parsed.cwd),
    })
}

fn scan_codex_session_dirs(root: &Path) -> Vec<NativeSessionSummary> {
    scan_session_dirs(root, "codex")
}

fn scan_session_dirs(root: &Path, provider: &str) -> Vec<NativeSessionSummary> {
    if !root.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let path = entry.path();
            let meta = fs::metadata(&path).ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            let title = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| provider.to_string());
            Some(NativeSessionSummary {
                provider: provider.to_string(),
                path: path.to_string_lossy().to_string(),
                title,
                updated_at: modified,
                session_id: None,
                cwd: None,
            })
        })
        .collect()
}

pub fn native_session_root(provider: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match provider {
        "claude" => Some(home.join(".claude").join("projects")),
        "codex" => Some(home.join(".codex").join("sessions")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn import_claude_session_builds_conversation_with_native_link() {
        let temp = tempfile::tempdir().unwrap();
        let jsonl = temp.path().join("session.jsonl");
        let mut file = fs::File::create(&jsonl).unwrap();
        writeln!(
            file,
            r#"{{"type":"user","sessionId":"abc-123","cwd":"/tmp/work","message":{{"role":"user","content":"hello claude"}},"uuid":"u1"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","sessionId":"abc-123","cwd":"/tmp/work","message":{{"role":"assistant","content":[{{"type":"text","text":"hi there"}}]}},"uuid":"a1"}}"#
        )
        .unwrap();

        let conversation =
            import_native_session("claude", jsonl.to_str().unwrap(), "claude-native", "sonnet")
                .unwrap();
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(
            conversation.native_session.as_ref().map(|link| link.session_id.as_str()),
            Some("abc-123")
        );
    }
}
