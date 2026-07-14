use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::conversation::model::ConversationKind;
use crate::conversation::{
    Conversation, Id, McpServerRef, Message, NativeSessionLink, Root, WorkingDir,
};
use crate::{CoreError, Result};

pub const SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub schema_version: u32,
    pub id: Id,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub kind: ConversationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_harness_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub working_dir: WorkingDir,
    pub mcp_servers: Vec<McpServerRef>,
    pub roots: Vec<Root>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session: Option<NativeSessionLink>,
}

impl ConversationMeta {
    pub fn from_conversation(c: &Conversation) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: c.id,
            title: c.title.clone(),
            created_at: c.created_at,
            updated_at: c.updated_at,
            project_id: c.project_id,
            kind: c.kind,
            active_harness_id: c.active_harness_id.clone(),
            model_id: c.model_id.clone(),
            working_dir: c.working_dir.clone(),
            mcp_servers: c.mcp_servers.clone(),
            roots: c.roots.clone(),
            forked_from: c.forked_from,
            native_session: c.native_session.clone(),
        }
    }

    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(input: &str) -> Result<Self> {
        let mut meta: Self = serde_json::from_str(input)?;
        if meta.schema_version > SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(meta.schema_version));
        }
        meta.roots = meta
            .roots
            .into_iter()
            .map(crate::conversation::Root::normalize_legacy)
            .collect();
        Ok(meta)
    }
}

pub fn message_to_line(message: &Message) -> Result<String> {
    for block in &message.content {
        block.validate()?;
    }

    let line = serde_json::to_string(message)?;
    if line.contains('\n') {
        return Err(CoreError::MalformedVault(
            "message serialization produced a newline".to_string(),
        ));
    }
    Ok(line)
}

pub fn message_from_line(line: &str) -> Result<Message> {
    let message: Message = serde_json::from_str(line)?;
    for block in &message.content {
        block.validate()?;
    }
    Ok(message)
}

impl Conversation {
    pub fn from_parts(meta: ConversationMeta, messages: Vec<Message>) -> Self {
        Self {
            id: meta.id,
            title: meta.title,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            project_id: meta.project_id,
            kind: meta.kind,
            active_harness_id: meta.active_harness_id,
            model_id: meta.model_id,
            working_dir: meta.working_dir,
            mcp_servers: meta.mcp_servers,
            roots: meta.roots,
            forked_from: meta.forked_from,
            native_session: meta.native_session,
            messages,
        }
    }
}
