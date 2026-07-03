use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::conversation::{
    Conversation, ConversationMeta, Id, Message, message_from_line, message_to_line,
};
use crate::vault::events::{Event, event_from_line, event_to_line};
use crate::vault::naming::folder_name;
use crate::vault::{ConversationSummary, ConversationVault, VaultIssue};
use crate::{CoreError, Result};

const LOCK_RETRY_FOR: Duration = Duration::from_secs(2);
const LOCK_RETRY_EVERY: Duration = Duration::from_millis(25);

#[derive(Debug, Clone)]
pub struct FilesystemVault {
    root: PathBuf,
}

#[derive(Debug, Clone)]
struct FolderEntry {
    path: PathBuf,
    meta: ConversationMeta,
}

#[derive(Debug)]
struct LockedConversation {
    file: File,
}

impl FilesystemVault {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("conversations"))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn append_vault_event(&self, event: &Event) -> Result<()> {
        append_vault_event(&self.root, event)
    }

    pub fn read_vault_events(&self) -> Result<Vec<Event>> {
        read_vault_events(&self.root)
    }

    pub fn conversation_workdir(&self, id: Id) -> Result<Option<PathBuf>> {
        let dir = self.resolve_folder(id)?;
        Ok(Some(dir.join("workdir")))
    }

    pub fn conversation_folder(&self, id: Id) -> Result<PathBuf> {
        self.resolve_folder(id)
    }

    pub fn meta_updated_at(&self, id: Id) -> Result<chrono::DateTime<chrono::Utc>> {
        let dir = self.resolve_folder(id)?;
        Ok(Self::read_meta_at(&dir)?.updated_at)
    }

    fn conversations_dir(&self) -> PathBuf {
        self.root.join("conversations")
    }

    fn conversation_dir(&self, c: &Conversation) -> PathBuf {
        self.conversations_dir().join(folder_name(c))
    }

    fn read_meta_at(path: &Path) -> Result<ConversationMeta> {
        let raw = fs::read_to_string(path.join("meta.json"))?;
        ConversationMeta::from_json(&raw)
    }

    fn scan_entries(&self) -> Result<Vec<FolderEntry>> {
        let mut entries = Vec::new();
        for item in fs::read_dir(self.conversations_dir())? {
            let item = item?;
            let path = item.path();
            if !path.is_dir() {
                continue;
            }
            if let Ok(meta) = Self::read_meta_at(&path) {
                entries.push(FolderEntry { path, meta });
            }
        }
        Ok(entries)
    }

    fn scan_issues_lossy(&self) -> Vec<VaultIssue> {
        let mut issues = Vec::new();
        let mut by_id: HashMap<Id, Vec<FolderEntry>> = HashMap::new();

        let read_dir = match fs::read_dir(self.conversations_dir()) {
            Ok(read_dir) => read_dir,
            Err(err) => {
                issues.push(VaultIssue::UnreadableFolder {
                    path: self.conversations_dir(),
                    reason: err.to_string(),
                });
                return issues;
            }
        };

        for item in read_dir {
            let item = match item {
                Ok(item) => item,
                Err(err) => {
                    issues.push(VaultIssue::UnreadableFolder {
                        path: self.conversations_dir(),
                        reason: err.to_string(),
                    });
                    continue;
                }
            };
            let path = item.path();
            if !path.is_dir() {
                continue;
            }
            match Self::read_meta_at(&path) {
                Ok(meta) => {
                    let id = meta.id;
                    by_id
                        .entry(id)
                        .or_default()
                        .push(FolderEntry { path, meta });
                }
                Err(err) => issues.push(VaultIssue::UnreadableFolder {
                    path,
                    reason: err.to_string(),
                }),
            }
        }

        for (id, mut entries) in by_id {
            if entries.len() <= 1 {
                continue;
            }
            sort_entries_for_resolution(&mut entries);
            let winner = entries.remove(0).path;
            let losers = entries.into_iter().map(|entry| entry.path).collect();
            issues.push(VaultIssue::DuplicateId { id, winner, losers });
        }

        issues
    }

    fn resolve_folder(&self, id: Id) -> Result<PathBuf> {
        let mut matches = Vec::new();
        for item in fs::read_dir(self.conversations_dir())? {
            let item = item?;
            let path = item.path();
            if !path.is_dir() {
                continue;
            }
            let raw = match fs::read_to_string(path.join("meta.json")) {
                Ok(raw) => raw,
                Err(_) => continue,
            };
            match ConversationMeta::from_json(&raw) {
                Ok(meta) if meta.id == id => matches.push(FolderEntry { path, meta }),
                Ok(_) => {}
                Err(CoreError::UnsupportedSchemaVersion(version)) => {
                    if raw_meta_id(&raw)? == Some(id) {
                        return Err(CoreError::UnsupportedSchemaVersion(version));
                    }
                }
                Err(_) => {}
            }
        }
        if matches.is_empty() {
            return Err(CoreError::NotFound(id));
        }
        sort_entries_for_resolution(&mut matches);
        Ok(matches.remove(0).path)
    }

    fn write_meta_atomic(dir: &Path, c: &Conversation) -> Result<()> {
        let meta = ConversationMeta::from_conversation(c);
        let tmp = dir.join("meta.json.tmp");
        fs::write(&tmp, meta.to_json_pretty()?)?;
        fs::rename(tmp, dir.join("meta.json"))?;
        Ok(())
    }

    fn lock_for_dir(&self, id: Id, dir: &Path) -> Result<LockedConversation> {
        let path = dir.join("messages.jsonl");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let deadline = Instant::now() + LOCK_RETRY_FOR;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(LockedConversation { file }),
                Err(std::fs::TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(CoreError::ConversationBusy(id));
                    }
                    sleep(LOCK_RETRY_EVERY);
                }
                Err(std::fs::TryLockError::Error(err)) => return Err(CoreError::Io(err)),
            }
        }
    }

    fn lock_existing(&self, id: Id) -> Result<(PathBuf, LockedConversation)> {
        let dir = self.resolve_folder(id)?;
        let lock = self.lock_for_dir(id, &dir)?;
        Ok((dir, lock))
    }

    fn read_messages(dir: &Path) -> Result<(Vec<Message>, bool)> {
        let path = dir.join("messages.jsonl");
        let bytes = fs::read(&path)?;
        let text = String::from_utf8(bytes).map_err(|err| {
            CoreError::MalformedVault(format!("messages.jsonl is not UTF-8: {err}"))
        })?;
        parse_messages_with_torn_tail(&text)
    }

    fn read_events_from_dir(dir: &Path) -> Result<Vec<Event>> {
        let path = dir.join("events.jsonl");
        let bytes = fs::read(&path)?;
        let text = String::from_utf8(bytes).map_err(|err| {
            CoreError::MalformedVault(format!("events.jsonl is not UTF-8: {err}"))
        })?;
        let (events, _) = parse_events_with_torn_tail(&text)?;
        Ok(events)
    }

    fn load_from_dir(dir: &Path) -> Result<Conversation> {
        let meta = Self::read_meta_at(dir)?;
        let (messages, _) = Self::read_messages(dir)?;
        Ok(Conversation::from_parts(meta, messages))
    }
}

pub fn append_vault_event(root: &Path, event: &Event) -> Result<()> {
    let line = event_to_line(event)?;
    let path = root.join("events.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn read_vault_events(root: &Path) -> Result<Vec<Event>> {
    let path = root.join("events.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)?;
    raw.lines().map(event_from_line).collect()
}

impl ConversationVault for FilesystemVault {
    fn create(&self, c: &Conversation) -> Result<()> {
        let dir = self.conversation_dir(c);
        fs::create_dir_all(dir.join("attachments"))?;
        fs::create_dir_all(dir.join("workdir"))?;
        File::create(dir.join("events.jsonl"))?;
        File::create(dir.join("messages.jsonl"))?;

        let _lock = self.lock_for_dir(c.id, &dir)?;
        Self::write_meta_atomic(&dir, c)?;
        let mut messages = OpenOptions::new()
            .append(true)
            .open(dir.join("messages.jsonl"))?;
        for message in &c.messages {
            writeln!(messages, "{}", message_to_line(message)?)?;
        }
        Ok(())
    }

    fn save_meta(&self, c: &Conversation) -> Result<()> {
        let (dir, _lock) = self.lock_existing(c.id)?;
        Self::write_meta_atomic(&dir, c)
    }

    fn append_message(&self, id: Id, m: &Message) -> Result<()> {
        let (dir, mut lock) = self.lock_existing(id)?;
        repair_torn_tail(&mut lock.file)?;
        let mut conversation = Self::load_from_dir(&dir)?;
        conversation.updated_at = Utc::now();
        let mut messages = OpenOptions::new()
            .append(true)
            .open(dir.join("messages.jsonl"))?;
        writeln!(messages, "{}", message_to_line(m)?)?;
        Self::write_meta_atomic(&dir, &conversation)?;
        Ok(())
    }

    fn load(&self, id: Id) -> Result<Conversation> {
        let dir = self.resolve_folder(id)?;
        Self::load_from_dir(&dir)
    }

    fn list(&self) -> Result<Vec<ConversationSummary>> {
        let mut entries = self.scan_entries()?;
        sort_entries_for_resolution(&mut entries);
        let mut seen = HashMap::new();
        let mut summaries = Vec::new();
        for entry in entries {
            if seen.insert(entry.meta.id, ()).is_some() {
                continue;
            }
            summaries.push(ConversationSummary {
                id: entry.meta.id,
                title: entry.meta.title,
                updated_at: entry.meta.updated_at,
            });
        }
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.title.cmp(&b.title)));
        Ok(summaries)
    }

    fn delete(&self, id: Id) -> Result<()> {
        let (dir, _lock) = self.lock_existing(id)?;
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    fn import_folder_as_new(&self, src: &Path) -> Result<Conversation> {
        let imported = Self::load_from_dir(src)?;
        let mut new = imported.clone();
        new.id = uuid::Uuid::now_v7();
        new.created_at = Utc::now();
        new.updated_at = new.created_at;
        new.forked_from = None;
        let dir = self.conversation_dir(&new);
        self.create(&new)?;
        copy_attachments_dir(&src.join("attachments"), &dir.join("attachments"))?;
        Ok(new)
    }

    fn issues(&self) -> Result<Vec<VaultIssue>> {
        let mut issues = self.scan_issues_lossy();
        let entries = self.scan_entries()?;
        for entry in entries {
            match Self::read_messages(&entry.path) {
                Ok((_, true)) => issues.push(VaultIssue::TornTailDetected { id: entry.meta.id }),
                Ok((_, false)) => {}
                Err(err) => issues.push(VaultIssue::UnreadableFolder {
                    path: entry.path,
                    reason: err.to_string(),
                }),
            }
        }
        Ok(issues)
    }

    fn append_event(&self, id: Id, event: &Event) -> Result<()> {
        let (dir, mut lock) = self.lock_existing(id)?;
        repair_torn_tail_named(&mut lock.file, &dir.join("events.jsonl"))?;
        let mut events = OpenOptions::new()
            .append(true)
            .open(dir.join("events.jsonl"))?;
        writeln!(events, "{}", event_to_line(event)?)?;
        Ok(())
    }

    fn read_events(&self, id: Id) -> Result<Vec<Event>> {
        let dir = self.resolve_folder(id)?;
        Self::read_events_from_dir(&dir)
    }
}

fn raw_meta_id(raw: &str) -> Result<Option<Id>> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let Some(id) = value.get("id").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    id.parse()
        .map(Some)
        .map_err(|err| CoreError::MalformedVault(format!("invalid conversation id: {err}")))
}

fn sort_entries_for_resolution(entries: &mut [FolderEntry]) {
    entries.sort_by(|a, b| {
        b.meta
            .updated_at
            .cmp(&a.meta.updated_at)
            .then_with(|| a.path.cmp(&b.path))
    });
}

fn parse_messages_with_torn_tail(text: &str) -> Result<(Vec<Message>, bool)> {
    let mut messages = Vec::new();
    let mut torn_tail = false;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();

    for (idx, raw) in lines.iter().enumerate() {
        let is_last = idx == lines.len().saturating_sub(1);
        let newline_terminated = raw.ends_with('\n');
        let line = raw.trim_end_matches('\n');
        if line.is_empty() && newline_terminated {
            continue;
        }

        if !newline_terminated && is_last {
            torn_tail = true;
            continue;
        }

        match message_from_line(line) {
            Ok(message) => messages.push(message),
            Err(err) => {
                return Err(CoreError::MalformedVault(format!(
                    "malformed interior message line {}: {err}",
                    idx + 1
                )));
            }
        }
    }

    Ok((messages, torn_tail))
}

fn repair_torn_tail(file: &mut File) -> Result<()> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        file.seek(SeekFrom::End(0))?;
        return Ok(());
    }

    let last_newline = bytes.iter().rposition(|byte| *byte == b'\n');
    let truncate_to = last_newline.map_or(0, |idx| idx + 1);
    file.set_len(truncate_to as u64)?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

fn repair_torn_tail_named(lock_file: &mut File, target: &Path) -> Result<()> {
    if target.ends_with("messages.jsonl") {
        return repair_torn_tail(lock_file);
    }
    let mut file = OpenOptions::new().read(true).write(true).open(target)?;
    repair_torn_tail(&mut file)
}

fn copy_attachments_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_attachments_dir(&path, &target)?;
        } else {
            fs::copy(path, target)?;
        }
    }
    Ok(())
}

fn parse_events_with_torn_tail(text: &str) -> Result<(Vec<Event>, bool)> {
    let mut events = Vec::new();
    let mut torn_tail = false;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();

    for raw in lines {
        let newline_terminated = raw.ends_with('\n');
        let line = raw.trim_end_matches('\n');
        if line.is_empty() && newline_terminated {
            continue;
        }
        if !newline_terminated {
            torn_tail = true;
            continue;
        }
        events.push(event_from_line(line)?);
    }

    Ok((events, torn_tail))
}
