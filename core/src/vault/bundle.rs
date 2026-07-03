use std::fs::{self, File};
use std::path::Path;

use chrono::Utc;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::artifact::{verify_attachment, verify_inline_artifact};
use crate::conversation::{ContentBlock, Conversation, Id};
use crate::vault::fs::{copy_attachments_dir, FilesystemVault};
use crate::vault::ConversationVault;
use crate::{CoreError, Result};

const BUNDLE_META: &str = "meta.json";
const BUNDLE_MESSAGES: &str = "messages.jsonl";
const BUNDLE_ATTACHMENTS_PREFIX: &str = "attachments/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportWarning {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportResult {
    pub conversation: Conversation,
    pub warnings: Vec<ImportWarning>,
}

pub fn export_conversation_bundle(vault: &FilesystemVault, id: Id, dest: &Path) -> Result<()> {
    let dir = vault.conversation_folder(id)?;
    let conversation = vault.load(id)?;
    verify_conversation_attachments(&dir, &conversation)?;

    let file = File::create(dest)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    add_file_to_zip(&mut zip, &dir.join(BUNDLE_META), BUNDLE_META, options)?;
    add_file_to_zip(&mut zip, &dir.join(BUNDLE_MESSAGES), BUNDLE_MESSAGES, options)?;

    let attachments_dir = dir.join("attachments");
    if attachments_dir.is_dir() {
        for entry in fs::read_dir(&attachments_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let zip_path = format!("{BUNDLE_ATTACHMENTS_PREFIX}{name}");
                add_file_to_zip(&mut zip, &entry.path(), &zip_path, options)?;
            }
        }
    }

    zip.finish().map_err(zip_error)?;
    Ok(())
}

pub fn import_bundle_or_folder_as_new(
    vault: &FilesystemVault,
    src: &Path,
) -> Result<ImportResult> {
    if src.is_file() {
        import_from_zip(vault, src)
    } else if src.is_dir() {
        import_from_folder(vault, src)
    } else {
        Err(CoreError::MalformedVault(format!(
            "import source not found: {}",
            src.display()
        )))
    }
}

fn import_from_folder(vault: &FilesystemVault, src: &Path) -> Result<ImportResult> {
    let mut conversation = FilesystemVault::load_conversation_from_dir(src)?;
    let warnings = verify_and_mark_integrity(src, &mut conversation)?;
    let imported = finalize_import(vault, src, conversation)?;
    Ok(ImportResult {
        conversation: imported,
        warnings,
    })
}

fn import_from_zip(vault: &FilesystemVault, src: &Path) -> Result<ImportResult> {
    let temp = tempfile::tempdir()?;
    let extract_dir = temp.path().join("bundle");
    fs::create_dir_all(&extract_dir)?;
    extract_zip(src, &extract_dir)?;
    import_from_folder(vault, &extract_dir)
}

fn finalize_import(
    vault: &FilesystemVault,
    src: &Path,
    mut conversation: Conversation,
) -> Result<Conversation> {
    conversation.id = uuid::Uuid::now_v7();
    conversation.created_at = Utc::now();
    conversation.updated_at = conversation.created_at;
    conversation.forked_from = None;
    vault.create(&conversation)?;
    let dir = vault.conversation_folder(conversation.id)?;
    copy_attachments_dir(&src.join("attachments"), &dir.join("attachments"))?;
    Ok(conversation)
}

fn verify_conversation_attachments(dir: &Path, conversation: &Conversation) -> Result<()> {
    for message in &conversation.messages {
        for block in &message.content {
            if let ContentBlock::Artifact {
                size,
                sha256,
                inline,
                ..
            } = block
            {
                if block.integrity_failed() {
                    continue;
                }
                if let Some(text) = inline {
                    verify_inline_artifact(*size, sha256, text)?;
                } else {
                    verify_attachment(dir, block.artifact_path()?, *size, sha256)?;
                }
            }
        }
    }
    Ok(())
}

fn verify_and_mark_integrity(
    dir: &Path,
    conversation: &mut Conversation,
) -> Result<Vec<ImportWarning>> {
    let mut warnings = Vec::new();
    for message in &mut conversation.messages {
        for block in &mut message.content {
            let ContentBlock::Artifact {
                path,
                size,
                sha256,
                inline,
                integrity_failed,
                ..
            } = block
            else {
                continue;
            };
            if *integrity_failed {
                continue;
            }
            let path_str = path.clone();
            let verify_result = if let Some(text) = inline.as_deref() {
                verify_inline_artifact(*size, sha256, text)
            } else if !dir.join(&path_str).exists() {
                Err(CoreError::MalformedVault(format!(
                    "missing attachment file: {path_str}"
                )))
            } else {
                verify_attachment(dir, &path_str, *size, sha256).map(|_| ())
            };
            if let Err(err) = verify_result {
                *integrity_failed = true;
                warnings.push(ImportWarning {
                    kind: "integrity_failed".to_string(),
                    detail: format!("{path_str}: {err}"),
                });
            }
        }
    }
    Ok(warnings)
}

fn extract_zip(src: &Path, dest: &Path) -> Result<()> {
    let file = File::open(src)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let Some(safe_name) = sanitize_zip_entry_path(entry.name()) else {
            continue;
        };
        let out_path = dest.join(&safe_name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn sanitize_zip_entry_path(name: &str) -> Option<String> {
    let path = Path::new(name);
    if path.is_absolute() {
        return None;
    }
    for component in path.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return None;
        }
    }
    Some(name.to_string())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    source: &Path,
    zip_path: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(zip_path, options).map_err(zip_error)?;
    let mut file = File::open(source)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

fn zip_error(err: zip::result::ZipError) -> CoreError {
    CoreError::MalformedVault(format!("bundle zip error: {err}"))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use sha2::{Digest, Sha256};

    use crate::conversation::{ContentBlock, Conversation, Message, Role};
    use crate::vault::fs::FilesystemVault;
    use crate::vault::ConversationVault;

    use super::*;

    fn hex_sha256(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn sample_conversation() -> Conversation {
        let mut conversation = Conversation::new("Export me");
        conversation.active_harness_id = Some("mock-acp".to_string());
        conversation.model_id = Some("mock".to_string());
        conversation
    }

    fn write_conversation_with_extras(vault: &FilesystemVault, conversation: &Conversation) {
        vault.create(conversation).expect("create");
        let dir = vault.conversation_folder(conversation.id).expect("folder");
        fs::write(dir.join("events.jsonl"), "{\"kind\":\"test\"}\n").expect("events");
        fs::create_dir_all(dir.join("workdir")).expect("workdir");
        fs::write(dir.join("workdir/sales.csv"), "a,b\n1,2\n").expect("csv");
        fs::create_dir_all(dir.join("attachments")).expect("attachments");
        let html = "<h1>report</h1>";
        let sha256 = hex_sha256(html.as_bytes());
        fs::write(
            dir.join(format!("attachments/{sha256}-report.html")),
            html,
        )
        .expect("attachment");
        let message = Message {
            id: uuid::Uuid::now_v7(),
            role: Role::Assistant,
            harness_id: Some("mock-acp".to_string()),
            content: vec![ContentBlock::artifact(
                format!("attachments/{sha256}-report.html"),
                "text/html",
                html.len() as u64,
                sha256.clone(),
                None,
            )
            .expect("artifact")],
            created_at: Utc::now(),
        };
        vault.append_message(conversation.id, &message).expect("append");
    }

    #[test]
    fn export_bundle_excludes_events_and_workdir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault = FilesystemVault::new(temp.path()).expect("vault");
        let conversation = sample_conversation();
        write_conversation_with_extras(&vault, &conversation);

        let bundle_path = temp.path().join("export.tamtri");
        export_conversation_bundle(&vault, conversation.id, &bundle_path).expect("export");

        let file = File::open(&bundle_path).expect("open zip");
        let mut archive = ZipArchive::new(file).expect("archive");
        let names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .collect();
        assert!(names.iter().any(|name| name == BUNDLE_META));
        assert!(names.iter().any(|name| name == BUNDLE_MESSAGES));
        assert!(
            names
                .iter()
                .any(|name| name.starts_with(BUNDLE_ATTACHMENTS_PREFIX))
        );
        assert!(!names.iter().any(|name| name.contains("events.jsonl")));
        assert!(!names.iter().any(|name| name.contains("workdir")));
        assert!(!names.iter().any(|name| name.contains("sales.csv")));
    }

    #[test]
    fn import_bundle_hash_verifies_attachments() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault = FilesystemVault::new(temp.path()).expect("vault");
        let conversation = sample_conversation();
        write_conversation_with_extras(&vault, &conversation);
        let bundle_path = temp.path().join("roundtrip.tamtri");
        export_conversation_bundle(&vault, conversation.id, &bundle_path).expect("export");

        let imported = import_bundle_or_folder_as_new(&vault, &bundle_path).expect("import");
        assert_ne!(imported.conversation.id, conversation.id);
        assert!(imported.conversation.forked_from.is_none());
        assert!(imported.warnings.is_empty());
        let artifact = imported
            .conversation
            .messages
            .iter()
            .flat_map(|message| message.content.iter())
            .find(|block| matches!(block, ContentBlock::Artifact { .. }))
            .expect("artifact");
        assert!(!artifact.integrity_failed());
    }

    #[test]
    fn import_tampered_html_failed_integrity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault = FilesystemVault::new(temp.path()).expect("vault");
        let conversation = sample_conversation();
        write_conversation_with_extras(&vault, &conversation);
        let bundle_path = temp.path().join("tampered.tamtri");
        export_conversation_bundle(&vault, conversation.id, &bundle_path).expect("export");

        {
            let file = File::open(&bundle_path).expect("open");
            let mut archive = ZipArchive::new(file).expect("archive");
            let mut tampered = Vec::new();
            {
                let mut writer = ZipWriter::new(std::io::Cursor::new(&mut tampered));
                let options =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
                for index in 0..archive.len() {
                    let mut entry = archive.by_index(index).expect("entry");
                    let name = entry.name().to_string();
                    let mut bytes = Vec::new();
                    entry.read_to_end(&mut bytes).expect("read");
                    if name.starts_with(BUNDLE_ATTACHMENTS_PREFIX) && name.ends_with(".html") {
                        bytes = b"<html><body>tampered</body></html>".to_vec();
                    }
                    writer.start_file(name, options).expect("start");
                    writer.write_all(&bytes).expect("write");
                }
                writer.finish().expect("finish");
            }
            fs::write(&bundle_path, tampered).expect("write tampered");
        }

        let imported = import_bundle_or_folder_as_new(&vault, &bundle_path).expect("import");
        assert!(!imported.warnings.is_empty());
        let artifact = imported
            .conversation
            .messages
            .iter()
            .flat_map(|message| message.content.iter())
            .find(|block| matches!(block, ContentBlock::Artifact { .. }))
            .expect("artifact");
        assert!(artifact.integrity_failed());
    }
}
