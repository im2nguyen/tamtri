use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::conversation::TaskStatus;

/// MCP protocol task status (2025-11-25 experimental).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTaskStatus {
    Working,
    InputRequired,
    Completed,
    Failed,
    Cancelled,
}

impl McpTaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled
        )
    }
}

/// Optional progress surfaced on live task cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

/// Gateway-tracked task state for live cards and transcript reduction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskState {
    pub task_id: String,
    pub server_id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<TaskProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_status: Option<McpTaskStatus>,
}

/// Raw MCP task object from `tasks/get`, notifications, or create responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTask {
    pub task_id: String,
    pub status: McpTaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
}

pub const RELATED_TASK_META_KEY: &str = "io.modelcontextprotocol/related-task";

pub fn parse_create_task_result(value: &Value) -> Option<McpTask> {
    value
        .get("task")
        .and_then(parse_task_value)
        .or_else(|| parse_task_value(value))
}

pub fn parse_task_value(value: &Value) -> Option<McpTask> {
    let task_id = value
        .get("taskId")
        .or_else(|| value.get("task_id"))
        .and_then(Value::as_str)?
        .to_string();
    let status = value.get("status").and_then(parse_mcp_status)?;
    Some(McpTask {
        task_id,
        status,
        status_message: value
            .get("statusMessage")
            .or_else(|| value.get("status_message"))
            .and_then(Value::as_str)
            .map(str::to_string),
        poll_interval: value
            .get("pollInterval")
            .or_else(|| value.get("poll_interval"))
            .and_then(Value::as_u64),
        ttl: value.get("ttl").and_then(Value::as_u64),
    })
}

pub fn parse_mcp_status(value: &Value) -> Option<McpTaskStatus> {
    match value.as_str()? {
        "working" => Some(McpTaskStatus::Working),
        "input_required" => Some(McpTaskStatus::InputRequired),
        "completed" => Some(McpTaskStatus::Completed),
        "failed" => Some(McpTaskStatus::Failed),
        "cancelled" => Some(McpTaskStatus::Cancelled),
        _ => None,
    }
}

pub fn related_task_id_from_meta(meta: Option<&Value>) -> Option<String> {
    let meta = meta?;
    meta.get(RELATED_TASK_META_KEY)
        .or_else(|| meta.get("relatedTask"))
        .and_then(|value| value.get("taskId").or_else(|| value.get("task_id")))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn task_status_for_transcript(status: &McpTaskStatus) -> TaskStatus {
    match status {
        McpTaskStatus::Working | McpTaskStatus::InputRequired => TaskStatus::Running,
        McpTaskStatus::Completed => TaskStatus::Completed,
        McpTaskStatus::Failed | McpTaskStatus::Cancelled => TaskStatus::Failed,
    }
}

pub fn task_state_from_mcp(
    server_id: &str,
    task: &McpTask,
    origin_tool_call_id: Option<String>,
    title: Option<String>,
    result: Option<Value>,
) -> TaskState {
    TaskState {
        task_id: task.task_id.clone(),
        server_id: server_id.to_string(),
        status: task_status_for_transcript(&task.status),
        title,
        progress: task.status_message.as_ref().map(|message| TaskProgress {
            message: Some(message.clone()),
            percent: None,
        }),
        result,
        origin_tool_call_id,
        mcp_status: Some(task.status.clone()),
    }
}

pub fn default_poll_interval_ms(task: &McpTask) -> u64 {
    task.poll_interval.unwrap_or(500)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_create_task_result_from_nested_task() {
        let value = json!({
            "task": {
                "taskId": "t-1",
                "status": "working",
                "pollInterval": 250
            }
        });
        let task = parse_create_task_result(&value).expect("task");
        assert_eq!(task.task_id, "t-1");
        assert_eq!(task.status, McpTaskStatus::Working);
        assert_eq!(task.poll_interval, Some(250));
    }

    #[test]
    fn related_task_id_from_meta_reads_task_id() {
        let meta = json!({
            "io.modelcontextprotocol/related-task": {"taskId": "task-42"}
        });
        assert_eq!(
            related_task_id_from_meta(Some(&meta)).as_deref(),
            Some("task-42")
        );
    }
}
