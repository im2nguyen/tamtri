use chrono::Utc;

use crate::conversation::model::ConversationKind;
use crate::conversation::{Conversation, Message, WorkingDir};

impl Conversation {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::now_v7(),
            title: title.into(),
            created_at: now,
            updated_at: now,
            project_id: None,
            active_harness_id: None,
            model_id: None,
            working_dir: WorkingDir::VaultLocal,
            mcp_servers: Vec::new(),
            roots: Vec::new(),
            forked_from: None,
            native_session: None,
            kind: ConversationKind::User,
            messages: Vec::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn fork(&self) -> Self {
        let now = Utc::now();
        let mut fork = self.clone();
        fork.id = uuid::Uuid::now_v7();
        fork.forked_from = Some(self.id);
        fork.created_at = now;
        fork.updated_at = now;
        fork
    }

    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
        self.touch();
    }
}
