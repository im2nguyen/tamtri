use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};

use crate::conversation::reduce::TurnReducer;
use crate::conversation::{ContentBlock, Conversation, Id, Message, Role, WorkingDir};
use crate::harness::acp::{AcpAdapter, AgentLaunchSpec};
use crate::harness::{
    ContextSeed, ConversationContext, HarnessAdapter, HarnessEvent, RunControl, TurnInput,
};
use crate::vault::events::{Event, EventKind};
use crate::vault::fs::FilesystemVault;
use crate::vault::{ConversationSummary, ConversationVault};
use crate::{CoreError, Result};

#[uniffi::export(foreign)]
pub trait ConversationObserver: Send + Sync {
    fn on_event(&self, event: UiEvent);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct UiEvent {
    pub conversation_id: String,
    pub kind: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ConversationSummaryDto {
    pub id: String,
    pub title: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ConversationDto {
    pub id: String,
    pub title: String,
    pub active_harness_id: Option<String>,
    pub model_id: Option<String>,
    pub messages_json: Vec<String>,
}

type FfiResult<T> = std::result::Result<T, TamtriError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum TamtriError {
    #[error("{message}")]
    Core { message: String },
}

impl From<CoreError> for TamtriError {
    fn from(err: CoreError) -> Self {
        ffi_err(err)
    }
}

#[derive(uniffi::Object)]
pub struct TamtriCore {
    vault: Arc<FilesystemVault>,
    runtime: Runtime,
    adapters: Arc<Mutex<HashMap<String, Arc<dyn HarnessAdapter>>>>,
    active_runs: Arc<Mutex<HashMap<Id, RunControl>>>,
    observer: Arc<dyn ConversationObserver>,
}

#[uniffi::export]
impl TamtriCore {
    #[uniffi::constructor]
    pub fn new(vault_path: String, observer: Arc<dyn ConversationObserver>) -> FfiResult<Self> {
        Self::new_inner(vault_path.into(), observer).map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn new_inner(vault_path: PathBuf, observer: Arc<dyn ConversationObserver>) -> Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        Ok(Self {
            vault: Arc::new(FilesystemVault::new(vault_path)?),
            runtime,
            adapters: Arc::new(Mutex::new(HashMap::new())),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            observer,
        })
    }

    fn register_acp_agent_spec(&self, spec: AgentLaunchSpec) -> Result<()> {
        let adapter = Arc::new(AcpAdapter::new(spec.clone()));
        self.adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .insert(spec.id, adapter);
        Ok(())
    }
}

#[uniffi::export]
impl TamtriCore {
    pub fn register_acp_agent(
        &self,
        id: String,
        display_name: String,
        command: String,
        args: Vec<String>,
    ) -> FfiResult<()> {
        self.register_acp_agent_spec(AgentLaunchSpec {
            id,
            display_name,
            command,
            args,
            env: Vec::new(),
        })
        .map_err(ffi_err)
    }

    pub fn list_conversations(&self) -> FfiResult<Vec<ConversationSummaryDto>> {
        self.vault
            .list()?
            .into_iter()
            .map(summary_to_dto)
            .collect::<Result<Vec<_>>>()
            .map_err(ffi_err)
    }

    pub fn load_conversation(&self, id: String) -> FfiResult<ConversationDto> {
        self.load_conversation_inner(&id).map_err(ffi_err)
    }

    pub fn create_conversation(
        &self,
        title: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.create_conversation_inner(&title, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn fork_conversation(
        &self,
        id: String,
        harness_id: String,
        model_id: String,
    ) -> FfiResult<ConversationDto> {
        self.fork_conversation_inner(&id, &harness_id, &model_id)
            .map_err(ffi_err)
    }

    pub fn delete_conversation(&self, id: String) -> FfiResult<()> {
        self.vault.delete(parse_id(&id)?).map_err(ffi_err)
    }

    pub fn send_message(&self, conversation_id: String, text: String) -> FfiResult<()> {
        self.send_message_inner(&conversation_id, &text)
            .map_err(ffi_err)
    }

    pub fn respond_permission(
        &self,
        conversation_id: String,
        request_id: String,
        option_id: String,
    ) -> FfiResult<()> {
        self.respond_permission_inner(&conversation_id, &request_id, &option_id)
            .map_err(ffi_err)
    }

    pub fn cancel_run(&self, conversation_id: String) -> FfiResult<()> {
        self.cancel_run_inner(&conversation_id).map_err(ffi_err)
    }
}

impl TamtriCore {
    pub fn load_conversation_inner(&self, id: &str) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        conversation_to_dto(self.vault.load(id)?)
    }

    pub fn create_conversation_inner(
        &self,
        title: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let mut conversation = Conversation::new(title);
        conversation.active_harness_id = Some(harness_id.to_string());
        conversation.model_id = Some(model_id.to_string());
        self.vault.create(&conversation)?;
        conversation_to_dto(conversation)
    }

    pub fn fork_conversation_inner(
        &self,
        id: &str,
        harness_id: &str,
        model_id: &str,
    ) -> Result<ConversationDto> {
        let id = parse_id(id)?;
        let mut fork = self.vault.load(id)?.fork();
        fork.active_harness_id = Some(harness_id.to_string());
        fork.model_id = Some(model_id.to_string());
        self.vault.create(&fork)?;
        conversation_to_dto(fork)
    }

    pub fn send_message_inner(&self, conversation_id: &str, text: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        if self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .contains_key(&id)
        {
            return Err(CoreError::ConversationBusy(id));
        }

        let mut conversation = self.vault.load(id)?;
        let harness_id = conversation
            .active_harness_id
            .clone()
            .ok_or_else(|| CoreError::Protocol("conversation has no active harness".to_string()))?;
        let model_id = conversation
            .model_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let adapter = self.adapter(&harness_id)?;
        let user_message = Message {
            id: Id::now_v7(),
            role: Role::User,
            harness_id: None,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            created_at: Utc::now(),
        };
        self.vault.append_message(id, &user_message)?;
        conversation.messages.push(user_message.clone());
        let ctx = ConversationContext {
            seed: ContextSeed::FreshTranscript {
                messages: conversation.messages.clone(),
            },
            working_dir: WorkingDir::VaultLocal,
            working_dir_path: self
                .vault
                .conversation_workdir(id)?
                .unwrap_or_else(|| self.vault.root().join("conversations")),
            roots: conversation.roots.clone(),
            mcp_servers: conversation.mcp_servers.clone(),
            model_id,
        };

        self.vault.append_event(
            id,
            &Event::new(EventKind::TurnStarted, json!({ "harness_id": harness_id })),
        )?;

        let vault = Arc::clone(&self.vault);
        let active_runs = Arc::clone(&self.active_runs);
        let observer = Arc::clone(&self.observer);
        self.runtime.spawn(async move {
            let run = adapter.run(ctx, TurnInput { user_message }).await;
            match run {
                Ok(mut run) => {
                    if let Ok(mut runs) = active_runs.lock() {
                        runs.insert(id, run.control.clone());
                    }
                    let mut reducer = TurnReducer::new(harness_id.clone());
                    while let Some(event) = run.events.recv().await {
                        emit(&observer, id, &event);
                        let _ = append_event_for_harness_event(&vault, id, &event);
                        let _ = reducer.apply(&event);
                        if matches!(event, HarnessEvent::TurnEnded { .. }) {
                            let reduced = reducer.finish();
                            if !reduced.message.content.is_empty() {
                                let _ = vault.append_message(id, &reduced.message);
                                observer.on_event(UiEvent {
                                    conversation_id: id.to_string(),
                                    kind: "message_committed".to_string(),
                                    payload_json: serde_json::to_string(&reduced.message)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                });
                            }
                            break;
                        }
                    }
                    if let Ok(mut runs) = active_runs.lock() {
                        runs.remove(&id);
                    }
                }
                Err(err) => {
                    let _ = vault.append_event(
                        id,
                        &Event::new(EventKind::Error, json!({ "message": err.to_string() })),
                    );
                    observer.on_event(UiEvent {
                        conversation_id: id.to_string(),
                        kind: "error".to_string(),
                        payload_json: json!({ "message": err.to_string() }).to_string(),
                    });
                }
            }
        });
        Ok(())
    }

    pub fn respond_permission_inner(
        &self,
        conversation_id: &str,
        request_id: &str,
        option_id: &str,
    ) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let control = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .cloned()
            .ok_or(CoreError::NotFound(id))?;
        self.runtime
            .block_on(control.respond_permission(request_id, option_id))
    }

    pub fn cancel_run_inner(&self, conversation_id: &str) -> Result<()> {
        let id = parse_id(conversation_id)?;
        let control = self
            .active_runs
            .lock()
            .map_err(|_| CoreError::Protocol("run registry lock poisoned".to_string()))?
            .get(&id)
            .cloned()
            .ok_or(CoreError::NotFound(id))?;
        self.runtime.block_on(control.cancel())
    }

    fn adapter(&self, harness_id: &str) -> Result<Arc<dyn HarnessAdapter>> {
        self.adapters
            .lock()
            .map_err(|_| CoreError::Protocol("adapter registry lock poisoned".to_string()))?
            .get(harness_id)
            .cloned()
            .ok_or_else(|| CoreError::Protocol(format!("unknown harness: {harness_id}")))
    }
}

fn emit(observer: &Arc<dyn ConversationObserver>, conversation_id: Id, event: &HarnessEvent) {
    observer.on_event(UiEvent {
        conversation_id: conversation_id.to_string(),
        kind: event_kind(event).to_string(),
        payload_json: serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
    });
}

fn append_event_for_harness_event(
    vault: &FilesystemVault,
    id: Id,
    event: &HarnessEvent,
) -> Result<()> {
    let (kind, payload) = match event {
        HarnessEvent::ToolCallStarted {
            id, name, input, ..
        } => (
            EventKind::ToolCallStarted,
            json!({ "id": id, "name": name, "input": input }),
        ),
        HarnessEvent::ToolCallProgress {
            id,
            status,
            content,
        } => (
            EventKind::ToolCallCompleted,
            json!({ "id": id, "status": status, "content": content }),
        ),
        HarnessEvent::PermissionRequested {
            request_id,
            action,
            detail,
            options,
        } => (
            EventKind::PermissionRequested,
            json!({ "request_id": request_id, "action": action, "detail": detail, "options": options }),
        ),
        HarnessEvent::PermissionResolved {
            request_id,
            option_id,
        } => (
            EventKind::PermissionResolved,
            json!({ "request_id": request_id, "option_id": option_id }),
        ),
        HarnessEvent::TurnEnded { reason } => (EventKind::TurnEnded, json!({ "reason": reason })),
        HarnessEvent::Error { message } => (EventKind::Error, json!({ "message": message })),
        _ => return Ok(()),
    };
    vault.append_event(id, &Event::new(kind, payload))
}

fn event_kind(event: &HarnessEvent) -> &'static str {
    match event {
        HarnessEvent::TextDelta { .. } => "text_delta",
        HarnessEvent::ThoughtDelta { .. } => "thought_delta",
        HarnessEvent::ToolCallStarted { .. } => "tool_call_started",
        HarnessEvent::ToolCallProgress { .. } => "tool_call_progress",
        HarnessEvent::FileChanged { .. } => "file_changed",
        HarnessEvent::PermissionRequested { .. } => "permission_requested",
        HarnessEvent::PermissionResolved { .. } => "permission_resolved",
        HarnessEvent::TerminalOutput { .. } => "terminal_output",
        HarnessEvent::PlanUpdated { .. } => "plan_updated",
        HarnessEvent::ModeChanged { .. } => "mode_changed",
        HarnessEvent::Error { .. } => "error",
        HarnessEvent::TurnEnded { .. } => "turn_ended",
    }
}

fn parse_id(id: &str) -> Result<Id> {
    id.parse()
        .map_err(|err| CoreError::MalformedVault(format!("invalid conversation id: {err}")))
}

fn summary_to_dto(summary: ConversationSummary) -> Result<ConversationSummaryDto> {
    Ok(ConversationSummaryDto {
        id: summary.id.to_string(),
        title: summary.title,
        updated_at: summary.updated_at.to_rfc3339(),
    })
}

fn ffi_err(err: CoreError) -> TamtriError {
    TamtriError::Core {
        message: err.to_string(),
    }
}

fn conversation_to_dto(conversation: Conversation) -> Result<ConversationDto> {
    let messages_json = conversation
        .messages
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(ConversationDto {
        id: conversation.id.to_string(),
        title: conversation.title,
        active_harness_id: conversation.active_harness_id,
        model_id: conversation.model_id,
        messages_json,
    })
}
