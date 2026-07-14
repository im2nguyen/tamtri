//! Legible orchestration run records under `<vault>/orchestration/<run-id>/meta.json`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

pub const RUN_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl OrchestrationRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationRunMeta {
    pub schema_version: u32,
    pub id: String,
    pub recipe_id: String,
    pub source_conversation_id: String,
    pub status: OrchestrationRunStatus,
    pub current_step: u32,
    pub latest_conversation_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_conversation_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationRunDto {
    pub id: String,
    pub recipe_id: String,
    pub source_conversation_id: String,
    pub status: String,
    pub current_step: u32,
    pub latest_conversation_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_conversation_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub started_at: String,
    pub updated_at: String,
}

impl OrchestrationRunMeta {
    pub fn new(id: String, recipe_id: String, source_conversation_id: String) -> Self {
        let now = Utc::now();
        Self {
            schema_version: RUN_SCHEMA_VERSION,
            id: id.clone(),
            recipe_id,
            source_conversation_id: source_conversation_id.clone(),
            status: OrchestrationRunStatus::Running,
            current_step: 0,
            latest_conversation_id: source_conversation_id,
            branch_conversation_ids: Vec::new(),
            error: None,
            started_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn to_dto(&self) -> OrchestrationRunDto {
        OrchestrationRunDto {
            id: self.id.clone(),
            recipe_id: self.recipe_id.clone(),
            source_conversation_id: self.source_conversation_id.clone(),
            status: self.status.as_str().to_string(),
            current_step: self.current_step,
            latest_conversation_id: self.latest_conversation_id.clone(),
            branch_conversation_ids: self.branch_conversation_ids.clone(),
            error: self.error.clone(),
            started_at: self.started_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}
