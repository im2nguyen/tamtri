use std::path::{Component, Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Id = uuid::Uuid;

pub const ARTIFACT_INLINE_MAX_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Id,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_harness_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub working_dir: WorkingDir,
    pub mcp_servers: Vec<McpServerRef>,
    pub roots: Vec<Root>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<Id>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Id,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_id: Option<String>,
    pub content: Vec<ContentBlock>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        call_id: String,
        output: serde_json::Value,
    },
    AppResource {
        uri: String,
        template_ref: String,
        state: serde_json::Value,
    },
    Artifact {
        path: String,
        mime_type: String,
        size: u64,
        sha256: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        inline: Option<String>,
    },
    ElicitationRequest {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        server_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        origin_tool_call_id: Option<String>,
        mode: ElicitationMode,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    ElicitationResponse {
        request_id: String,
        action: ElicitationAction,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    TaskRef {
        task_id: String,
        status: TaskStatus,
    },
}

impl ContentBlock {
    pub fn artifact(
        path: impl Into<String>,
        mime_type: impl Into<String>,
        size: u64,
        sha256: impl Into<String>,
        inline: Option<String>,
    ) -> crate::Result<Self> {
        let block = ContentBlock::Artifact {
            path: path.into(),
            mime_type: mime_type.into(),
            size,
            sha256: sha256.into(),
            inline,
        };
        block.validate()?;
        Ok(block)
    }

    pub fn validate(&self) -> crate::Result<()> {
        if let ContentBlock::Artifact {
            path,
            mime_type,
            inline,
            ..
        } = self
        {
            validate_artifact_path(path)?;

            if let Some(text) = inline {
                if !is_inline_text_mime(mime_type) {
                    return Err(crate::CoreError::MalformedVault(format!(
                        "inline artifact must be UTF-8 text, got mime type: {mime_type}"
                    )));
                }
                if text.len() > ARTIFACT_INLINE_MAX_BYTES {
                    return Err(crate::CoreError::MalformedVault(format!(
                        "inline artifact exceeds {ARTIFACT_INLINE_MAX_BYTES} bytes"
                    )));
                }
            }
        }

        Ok(())
    }
}

fn validate_artifact_path(path: &str) -> crate::Result<()> {
    let path_ref = Path::new(path);
    if path_ref.is_absolute() {
        return Err(crate::CoreError::MalformedVault(format!(
            "artifact path must be vault-relative and under attachments/: {path}"
        )));
    }
    if path.split('/').any(|part| part == "." || part == "..") {
        return Err(crate::CoreError::MalformedVault(format!(
            "artifact path must not contain traversal components: {path}"
        )));
    }

    let mut components = path_ref.components();
    match components.next() {
        Some(Component::Normal(first)) if first == "attachments" => {}
        _ => {
            return Err(crate::CoreError::MalformedVault(format!(
                "artifact path must be vault-relative and under attachments/: {path}"
            )));
        }
    }

    let mut has_child = false;
    for component in components {
        match component {
            Component::Normal(_) => has_child = true,
            Component::ParentDir
            | Component::CurDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(crate::CoreError::MalformedVault(format!(
                    "artifact path must not contain traversal components: {path}"
                )));
            }
        }
    }

    if !has_child {
        return Err(crate::CoreError::MalformedVault(format!(
            "artifact path must name a file under attachments/: {path}"
        )));
    }

    Ok(())
}

fn is_inline_text_mime(mime_type: &str) -> bool {
    let mime_type = mime_type
        .split_once(';')
        .map_or(mime_type, |(base, _)| base)
        .trim()
        .to_ascii_lowercase();
    mime_type.starts_with("text/")
        || matches!(
            mime_type.as_str(),
            "application/json"
                | "application/xml"
                | "application/xhtml+xml"
                | "application/javascript"
                | "application/ecmascript"
                | "image/svg+xml"
        )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationMode {
    Form,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationAction {
    Accept,
    Decline,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum WorkingDir {
    #[default]
    VaultLocal,
    External {
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerRef {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
