//! Discover native harness session directories for import into the vault.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::protocol::WireU64;

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSessionSummary {
    pub provider: String,
    pub path: String,
    pub title: String,
    pub updated_at: WireU64,
}

pub fn list_native_sessions() -> Vec<NativeSessionSummary> {
    let mut out = Vec::new();
    if let Some(home) = dirs::home_dir() {
        out.extend(scan_claude_projects(&home.join(".claude").join("projects")));
        out.extend(scan_codex_sessions(&home.join(".codex").join("sessions")));
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
    out
}

fn scan_claude_projects(root: &Path) -> Vec<NativeSessionSummary> {
    scan_session_dirs(root, "claude")
}

fn scan_codex_sessions(root: &Path) -> Vec<NativeSessionSummary> {
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
