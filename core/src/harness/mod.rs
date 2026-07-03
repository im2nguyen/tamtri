pub mod acp;
pub mod health;

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::Result;
use crate::conversation::{Message, WorkingDir};

#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn capabilities(&self) -> HarnessCapabilities;
    async fn run(&self, ctx: ConversationContext, turn: TurnInput) -> Result<HarnessRun>;
    async fn available_models(&self) -> Result<Vec<ModelInfo>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HarnessCapabilities {
    pub streaming: bool,
    pub tools: bool,
    pub permissions: bool,
    pub thinking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationContext {
    pub seed: ContextSeed,
    pub working_dir: WorkingDir,
    pub working_dir_path: PathBuf,
    pub roots: Vec<crate::conversation::Root>,
    pub mcp_servers: Vec<crate::conversation::McpServerRef>,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextSeed {
    FreshTranscript { messages: Vec<Message> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnInput {
    pub user_message: Message,
}

pub struct HarnessRun {
    pub events: mpsc::Receiver<HarnessEvent>,
    pub control: RunControl,
}

#[derive(Clone)]
pub struct RunControl {
    command_tx: mpsc::Sender<RunCommand>,
}

#[derive(Debug, Clone)]
pub enum RunCommand {
    Cancel,
    RespondPermission {
        request_id: String,
        option_id: String,
    },
}

impl RunControl {
    pub fn new(command_tx: mpsc::Sender<RunCommand>) -> Self {
        Self { command_tx }
    }

    pub async fn cancel(&self) -> Result<()> {
        self.command_tx
            .send(RunCommand::Cancel)
            .await
            .map_err(|_| crate::CoreError::TransportClosed)
    }

    pub async fn respond_permission(&self, request_id: &str, option_id: &str) -> Result<()> {
        self.command_tx
            .send(RunCommand::RespondPermission {
                request_id: request_id.to_string(),
                option_id: option_id.to_string(),
            })
            .await
            .map_err(|_| crate::CoreError::TransportClosed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HarnessEvent {
    TextDelta {
        text: String,
    },
    ThoughtDelta {
        text: String,
    },
    ToolCallStarted {
        id: String,
        name: String,
        kind: ToolKind,
        title: String,
        input: serde_json::Value,
    },
    ToolCallProgress {
        id: String,
        status: ToolStatus,
        content: Vec<ToolContent>,
    },
    FileChanged {
        tool_call_id: String,
        path: String,
        change: FileChange,
        diff: Diff,
    },
    PermissionRequested {
        request_id: String,
        action: String,
        detail: PermissionDetail,
        options: Vec<PermissionOption>,
    },
    PermissionResolved {
        request_id: String,
        option_id: String,
    },
    TerminalOutput {
        tool_call_id: String,
        chunk: String,
    },
    PlanUpdated {
        steps: Vec<PlanStep>,
    },
    ModeChanged {
        mode: String,
    },
    Error {
        message: String,
    },
    TurnEnded {
        reason: TurnEndReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Write,
    Execute,
    Search,
    Fetch,
    Think,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    Text { text: String },
    Diff { diff: Diff },
    Json { value: serde_json::Value },
    ResourceRef { uri: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    pub path: String,
    pub change: FileChange,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChange {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionDetail {
    FileEdit { diff: Diff },
    Command { command: String },
    Other { value: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub title: String,
    pub status: PlanStepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEndReason {
    EndTurn,
    Cancelled,
    Failed,
    MaxTokens,
}
