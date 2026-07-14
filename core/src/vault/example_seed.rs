//! Seed a labeled pre-recorded example conversation on first vault init.

use std::fs;
use std::path::Path;

use chrono::Utc;
use uuid::Uuid;

use crate::Result;
use crate::conversation::{
    ContentBlock, Conversation, ConversationKind, Message, Role, WorkingDir,
};
use crate::vault::ConversationVault;
use crate::vault::fs::FilesystemVault;

const EXAMPLE_TITLE: &str = "Example: Spreadsheet to report";
const EXAMPLE_REPORT_HTML: &str = include_str!("../../assets/example/report.html");
const EXAMPLE_FLAG: &str = ".example-seeded";

pub fn seed_example_conversation_if_missing(
    vault_root: &Path,
    vault: &FilesystemVault,
) -> Result<()> {
    let flag = vault_root.join(EXAMPLE_FLAG);
    if flag.exists() {
        return Ok(());
    }
    if vault
        .list()?
        .iter()
        .any(|summary| summary.title.starts_with("Example:"))
    {
        fs::write(&flag, Utc::now().to_rfc3339())?;
        return Ok(());
    }

    let mut conversation = Conversation::new(EXAMPLE_TITLE);
    conversation.kind = ConversationKind::Example;
    conversation.working_dir = WorkingDir::VaultLocal;

    let user_id = Uuid::now_v7();
    let assistant_id = Uuid::now_v7();
    let now = Utc::now();

    conversation.messages.push(Message {
        id: user_id,
        role: Role::User,
        harness_id: None,
        content: vec![ContentBlock::Text {
            text: "Turn this CSV into a clear, self-contained report.".into(),
        }],
        created_at: now,
    });

    conversation.messages.push(Message {
        id: assistant_id,
        role: Role::Assistant,
        harness_id: Some("example".into()),
        content: vec![
            ContentBlock::Text {
                text: "Here is your report. Open it in the panel on the right.".into(),
            },
            ContentBlock::Artifact {
                path: "attachments/report.html".into(),
                mime_type: "text/html".into(),
                size: EXAMPLE_REPORT_HTML.len() as u64,
                sha256: sha256_hex(EXAMPLE_REPORT_HTML.as_bytes()),
                inline: None,
                integrity_failed: false,
            },
        ],
        created_at: now,
    });

    vault.create(&conversation)?;
    let dir = vault.conversation_folder(conversation.id)?;
    fs::create_dir_all(dir.join("attachments"))?;
    fs::write(dir.join("attachments/report.html"), EXAMPLE_REPORT_HTML)?;
    fs::write(&flag, conversation.id.to_string())?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::ConversationVault;
    use tempfile::tempdir;

    #[test]
    fn seeds_example_once() {
        let dir = tempdir().unwrap();
        let vault = FilesystemVault::new(dir.path().to_path_buf()).unwrap();
        seed_example_conversation_if_missing(dir.path(), &vault).unwrap();
        seed_example_conversation_if_missing(dir.path(), &vault).unwrap();
        let summaries = vault.list().unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].title.starts_with("Example:"));
    }
}
