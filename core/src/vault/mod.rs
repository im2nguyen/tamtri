pub mod bundle;
pub mod events;
pub mod fs;
pub mod naming;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::Result;
use crate::conversation::{Conversation, Id, Message};
use crate::vault::events::Event;

#[derive(Debug, Clone, PartialEq)]
pub struct ConversationSummary {
    pub id: Id,
    pub title: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VaultIssue {
    DuplicateId {
        id: Id,
        winner: PathBuf,
        losers: Vec<PathBuf>,
    },
    TornTailDetected {
        id: Id,
    },
    UnreadableFolder {
        path: PathBuf,
        reason: String,
    },
}

pub trait ConversationVault {
    fn create(&self, c: &Conversation) -> Result<()>;
    fn save_meta(&self, c: &Conversation) -> Result<()>;
    fn append_message(&self, id: Id, m: &Message) -> Result<()>;
    fn load(&self, id: Id) -> Result<Conversation>;
    fn list(&self) -> Result<Vec<ConversationSummary>>;
    fn delete(&self, id: Id) -> Result<()>;
    fn import_folder_as_new(&self, src: &Path) -> Result<Conversation>;
    fn issues(&self) -> Result<Vec<VaultIssue>>;
    fn append_event(&self, id: Id, event: &Event) -> Result<()>;
    fn read_events(&self, id: Id) -> Result<Vec<Event>>;
}
