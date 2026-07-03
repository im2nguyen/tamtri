use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::conversation::{ARTIFACT_INLINE_MAX_BYTES, ContentBlock};
use crate::harness::Diff;
use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactSnapshot {
    pub block: ContentBlock,
    pub original_path: PathBuf,
    pub attachment_path: String,
    pub mime_type: String,
    pub size: u64,
    pub sha256: String,
}

pub struct ArtifactSnapshotter {
    workdir: PathBuf,
    conversation_dir: PathBuf,
}

impl ArtifactSnapshotter {
    pub fn new(workdir: impl Into<PathBuf>, conversation_dir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
            conversation_dir: conversation_dir.into(),
        }
    }

    pub fn snapshot_file_changed(&self, diff: &Diff) -> Result<Option<ArtifactSnapshot>> {
        self.snapshot_relative_path(Path::new(&diff.path))
    }

    pub fn snapshot_referenced_paths<'a, I>(&self, paths: I) -> Result<Vec<ArtifactSnapshot>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut snapshots = Vec::new();
        let mut seen = HashSet::new();
        for path in paths {
            if !seen.insert(path.to_string()) {
                continue;
            }
            if let Some(snapshot) = self.snapshot_relative_path(Path::new(path))? {
                snapshots.push(snapshot);
            }
        }
        Ok(snapshots)
    }

    fn snapshot_relative_path(&self, path: &Path) -> Result<Option<ArtifactSnapshot>> {
        let original_path = self.resolve_workdir_path(path)?;
        if !original_path.exists() {
            return Ok(None);
        }
        if !original_path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&original_path)?;
        let Some(mime_type) = detect_mime(&original_path, &bytes) else {
            return Ok(None);
        };
        let sha256 = hex_sha256(&bytes);
        let basename = original_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(safe_basename)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "artifact".to_string());
        let attachment_path = format!("attachments/{}-{basename}", &sha256[..16]);
        let attachment_abs = self.conversation_dir.join(&attachment_path);
        fs::create_dir_all(
            attachment_abs
                .parent()
                .ok_or_else(|| CoreError::Protocol("attachment path missing parent".to_string()))?,
        )?;
        if attachment_abs.exists() {
            let existing = fs::read(&attachment_abs)?;
            if existing != bytes {
                return Err(CoreError::MalformedVault(format!(
                    "attachment collision for {attachment_path}"
                )));
            }
        } else {
            fs::write(&attachment_abs, &bytes)?;
        }
        let inline = inline_text(&mime_type, &bytes)?;
        let block = ContentBlock::artifact(
            attachment_path.clone(),
            mime_type.clone(),
            bytes.len() as u64,
            sha256.clone(),
            inline,
        )?;
        Ok(Some(ArtifactSnapshot {
            block,
            original_path,
            attachment_path,
            mime_type,
            size: bytes.len() as u64,
            sha256,
        }))
    }

    fn resolve_workdir_path(&self, path: &Path) -> Result<PathBuf> {
        if path.is_absolute() {
            return Err(CoreError::MalformedVault(
                "artifact source path must be relative to workdir".to_string(),
            ));
        }
        for component in path.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(CoreError::MalformedVault(
                    "artifact source path must not contain traversal".to_string(),
                ));
            }
        }
        Ok(self.workdir.join(path))
    }
}

pub fn detect_mime(path: &Path, bytes: &[u8]) -> Option<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if bytes.contains(&0) {
        return match ext.as_str() {
            "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Some("image/png".to_string()),
            "jpg" | "jpeg" if bytes.starts_with(b"\xff\xd8\xff") => Some("image/jpeg".to_string()),
            "gif" if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") => {
                Some("image/gif".to_string())
            }
            "webp" if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") => {
                Some("image/webp".to_string())
            }
            _ => None,
        };
    }
    match ext.as_str() {
        "html" | "htm" => Some("text/html".to_string()),
        "md" | "markdown" => Some("text/markdown".to_string()),
        "csv" => Some("text/csv".to_string()),
        "tsv" => Some("text/tab-separated-values".to_string()),
        "txt" => Some("text/plain".to_string()),
        "json" => Some("application/json".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Some("image/png".to_string()),
        "jpg" | "jpeg" if bytes.starts_with(b"\xff\xd8\xff") => Some("image/jpeg".to_string()),
        "gif" if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") => {
            Some("image/gif".to_string())
        }
        "webp" if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") => {
            Some("image/webp".to_string())
        }
        _ if looks_like_html(bytes) => Some("text/plain".to_string()),
        _ if std::str::from_utf8(bytes).is_ok() => Some("text/plain".to_string()),
        _ => None,
    }
}

pub fn verify_attachment(
    conversation_dir: &Path,
    path: &str,
    size: u64,
    sha256: &str,
) -> Result<PathBuf> {
    let block = ContentBlock::artifact(path.to_string(), "text/plain", size, sha256, None)?;
    block.validate()?;
    let abs = conversation_dir.join(path);
    let bytes = fs::read(&abs)?;
    if bytes.len() as u64 != size {
        return Err(CoreError::MalformedVault(format!(
            "artifact size mismatch for {path}"
        )));
    }
    if hex_sha256(&bytes) != sha256 {
        return Err(CoreError::MalformedVault(format!(
            "artifact hash mismatch for {path}"
        )));
    }
    Ok(abs)
}

pub fn verify_inline_artifact(size: u64, sha256: &str, inline: &str) -> Result<()> {
    let bytes = inline.as_bytes();
    if bytes.len() as u64 != size {
        return Err(CoreError::MalformedVault(format!(
            "inline artifact size mismatch: expected {size}, got {}",
            bytes.len()
        )));
    }
    if hex_sha256(bytes) != sha256 {
        return Err(CoreError::MalformedVault(
            "inline artifact hash mismatch".to_string(),
        ));
    }
    Ok(())
}

fn inline_text(mime_type: &str, bytes: &[u8]) -> Result<Option<String>> {
    if bytes.len() > ARTIFACT_INLINE_MAX_BYTES || !is_text_like(mime_type) {
        return Ok(None);
    }
    let text = std::str::from_utf8(bytes).map_err(|err| {
        CoreError::MalformedVault(format!("text artifact is not valid UTF-8: {err}"))
    })?;
    Ok(Some(text.to_string()))
}

fn is_text_like(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
        || matches!(
            mime_type,
            "application/json" | "application/xml" | "image/svg+xml"
        )
}

fn looks_like_html(bytes: &[u8]) -> bool {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).to_ascii_lowercase();
    prefix.contains("<!doctype html") || prefix.contains("<html")
}

fn safe_basename(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use crate::harness::FileChange;

    use super::*;

    #[test]
    fn snapshots_small_html_from_workdir() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("report.html"), "<h1>ok</h1>").unwrap();
        let snapshotter = ArtifactSnapshotter::new(&workdir, &convo);
        let snapshot = snapshotter
            .snapshot_file_changed(&Diff {
                path: "report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.mime_type, "text/html");
        assert!(snapshot.attachment_path.starts_with("attachments/"));
        assert!(convo.join(&snapshot.attachment_path).exists());
        assert!(matches!(
            snapshot.block,
            ContentBlock::Artifact {
                inline: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn snapshots_only_referenced_paths() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(workdir.join("nested")).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("sales.csv"), "region,revenue\nNorth,10\n").unwrap();
        fs::write(workdir.join("nested").join("report.html"), "<h1>ok</h1>").unwrap();
        fs::write(workdir.join("unknown.bin"), b"\0\0\0").unwrap();

        let mut snapshots = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_referenced_paths(["nested/report.html"])
            .unwrap();
        snapshots.sort_by(|a, b| a.attachment_path.cmp(&b.attachment_path));
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].mime_type, "text/html");
        assert_eq!(
            snapshots[0].original_path,
            workdir.join("nested").join("report.html")
        );
        assert!(convo.join(&snapshots[0].attachment_path).exists());
    }

    #[test]
    fn snapshot_referenced_paths_covers_all_renderable_variants() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("report.html"), "<h1>ok</h1>").unwrap();
        fs::write(workdir.join("notes.md"), "# Title").unwrap();
        fs::write(workdir.join("sales.csv"), "a,b\n1,2\n").unwrap();
        fs::write(
            workdir.join("icon.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
        )
        .unwrap();
        fs::write(workdir.join("unknown.bin"), b"\0\0\0").unwrap();

        let mut snapshots = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_referenced_paths([
                "report.html",
                "notes.md",
                "sales.csv",
                "icon.svg",
                "unknown.bin",
                "missing.txt",
            ])
            .unwrap();
        snapshots.sort_by(|a, b| a.mime_type.cmp(&b.mime_type));

        assert_eq!(snapshots.len(), 4);
        let mime_types: Vec<_> = snapshots.iter().map(|snapshot| snapshot.mime_type.as_str()).collect();
        assert_eq!(
            mime_types,
            vec!["image/svg+xml", "text/csv", "text/html", "text/markdown"]
        );
        for snapshot in &snapshots {
            assert!(snapshot.attachment_path.starts_with("attachments/"));
            assert!(snapshot.attachment_path.contains('-'));
            assert_eq!(snapshot.size, fs::metadata(&snapshot.original_path).unwrap().len());
            assert_eq!(
                snapshot.sha256,
                hex_sha256(&fs::read(&snapshot.original_path).unwrap())
            );
            assert!(convo.join(&snapshot.attachment_path).exists());
        }
    }

    #[test]
    fn snapshot_referenced_paths_deduplicates_paths() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("report.html"), "<h1>ok</h1>").unwrap();

        let snapshots = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_referenced_paths(["report.html", "report.html", "report.html"])
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].mime_type, "text/html");
    }

    #[test]
    fn snapshots_from_external_workdir_path() {
        let temp = tempfile::tempdir().unwrap();
        let external = temp.path().join("external-project");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&external).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(external.join("report.html"), "<h1>external</h1>").unwrap();

        let snapshot = ArtifactSnapshotter::new(&external, &convo)
            .snapshot_file_changed(&Diff {
                path: "report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.mime_type, "text/html");
        assert_eq!(snapshot.original_path, external.join("report.html"));
        assert!(convo.join(&snapshot.attachment_path).exists());
    }

    #[test]
    fn missing_source_file_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();

        let snapshot = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_file_changed(&Diff {
                path: "missing.html".to_string(),
                change: FileChange::Created,
                old_text: None,
                new_text: None,
            })
            .unwrap();
        assert!(snapshot.is_none());
    }

    #[test]
    fn stable_attachment_name_and_hash_for_same_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("report.html"), "<h1>ok</h1>").unwrap();
        let snapshotter = ArtifactSnapshotter::new(&workdir, &convo);
        let first = snapshotter
            .snapshot_file_changed(&Diff {
                path: "report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap()
            .unwrap();
        let second = snapshotter
            .snapshot_file_changed(&Diff {
                path: "report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap()
            .unwrap();
        assert_eq!(first.attachment_path, second.attachment_path);
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.size, second.size);
    }

    #[test]
    fn referenced_paths_ignore_unreferenced_workdir_files() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(workdir.join("sales.csv"), "region,revenue\nNorth,10\n").unwrap();
        fs::write(workdir.join("report.html"), "<h1>ok</h1>").unwrap();

        let snapshots = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_referenced_paths(["report.html"])
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].mime_type, "text/html");
    }

    #[test]
    fn rejects_source_path_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let snapshotter = ArtifactSnapshotter::new(temp.path(), temp.path());
        let err = snapshotter
            .snapshot_file_changed(&Diff {
                path: "../report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap_err();
        assert!(matches!(err, CoreError::MalformedVault(_)));
    }

    #[test]
    fn mismatched_html_txt_renders_as_text() {
        assert_eq!(
            detect_mime(Path::new("report.txt"), b"<html><body>x</body></html>").unwrap(),
            "text/plain"
        );
    }

    #[test]
    fn detect_mime_by_extension_and_sniffing() {
        let png_header = b"\x89PNG\r\n\x1a\n";
        let cases = [
            ("report.html", b"<html><body>x</body></html>" as &[u8], "text/html"),
            ("notes.md", b"# Title", "text/markdown"),
            ("sales.csv", b"a,b\n1,2", "text/csv"),
            ("data.tsv", b"a\tb", "text/tab-separated-values"),
            ("config.json", b"{\"ok\":true}", "application/json"),
            (
                "icon.svg",
                br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
                "image/svg+xml",
            ),
            ("photo.png", png_header, "image/png"),
            ("photo.jpg", b"\xff\xd8\xffabc", "image/jpeg"),
            ("anim.gif", b"GIF89a", "image/gif"),
            ("plain.txt", b"hello", "text/plain"),
        ];
        for (name, bytes, expected) in cases {
            assert_eq!(
                detect_mime(Path::new(name), bytes).as_deref(),
                Some(expected),
                "case: {name}"
            );
        }
    }

    #[test]
    fn detect_mime_rejects_binary_html_and_unknown_binary() {
        assert!(detect_mime(Path::new("report.html"), b"\0\0\0").is_none());
        assert!(detect_mime(Path::new("unknown.bin"), b"\0\0\0").is_none());
    }

    #[test]
    fn detect_mime_sniffs_png_magic_without_extension() {
        let bytes = b"\x89PNG\r\n\x1a\nrest";
        assert_eq!(
            detect_mime(Path::new("data.png"), bytes).as_deref(),
            Some("image/png")
        );
    }

    #[test]
    fn verify_inline_artifact_accepts_matching_content() {
        let inline = "<h1>ok</h1>";
        let sha256 = hex_sha256(inline.as_bytes());
        verify_inline_artifact(inline.len() as u64, &sha256, inline).unwrap();
    }

    #[test]
    fn verify_inline_artifact_rejects_hash_mismatch() {
        let inline = "<h1>ok</h1>";
        let err = verify_inline_artifact(inline.len() as u64, "deadbeef", inline).unwrap_err();
        assert!(matches!(err, CoreError::MalformedVault(_)));
    }

    #[test]
    fn verify_inline_artifact_rejects_size_mismatch() {
        let inline = "<h1>ok</h1>";
        let sha256 = hex_sha256(inline.as_bytes());
        let err = verify_inline_artifact((inline.len() as u64) + 1, &sha256, inline).unwrap_err();
        assert!(matches!(err, CoreError::MalformedVault(_)));
    }

    #[test]
    fn does_not_inline_large_html() {
        let temp = tempfile::tempdir().unwrap();
        let workdir = temp.path().join("workdir");
        let convo = temp.path().join("conversation");
        fs::create_dir_all(&workdir).unwrap();
        fs::create_dir_all(convo.join("attachments")).unwrap();
        let large = format!("<h1>{}</h1>", "x".repeat(40_000));
        fs::write(workdir.join("report.html"), &large).unwrap();
        let snapshot = ArtifactSnapshotter::new(&workdir, &convo)
            .snapshot_file_changed(&Diff {
                path: "report.html".to_string(),
                change: FileChange::Modified,
                old_text: None,
                new_text: None,
            })
            .unwrap()
            .unwrap();
        assert!(matches!(
            snapshot.block,
            ContentBlock::Artifact { inline: None, .. }
        ));
    }

    #[test]
    fn verify_attachment_rejects_hash_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let convo = temp.path().join("conversation");
        fs::create_dir_all(convo.join("attachments")).unwrap();
        fs::write(convo.join("attachments/a.txt"), b"hello").unwrap();
        let err = verify_attachment(&convo, "attachments/a.txt", 5, "deadbeef").unwrap_err();
        assert!(matches!(err, CoreError::MalformedVault(_)));
    }

    #[test]
    fn verify_attachment_accepts_matching_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let convo = temp.path().join("conversation");
        fs::create_dir_all(convo.join("attachments")).unwrap();
        let bytes = b"hello";
        fs::write(convo.join("attachments/a.txt"), bytes).unwrap();
        let sha256 = hex_sha256(bytes);
        let path = verify_attachment(&convo, "attachments/a.txt", bytes.len() as u64, &sha256)
            .unwrap();
        assert_eq!(path, convo.join("attachments/a.txt"));
    }

    #[test]
    fn verify_attachment_rejects_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let convo = temp.path().join("conversation");
        fs::create_dir_all(convo.join("attachments")).unwrap();
        let err = verify_attachment(&convo, "attachments/missing.txt", 0, "abc").unwrap_err();
        assert!(matches!(err, CoreError::Io(_)));
    }
}
