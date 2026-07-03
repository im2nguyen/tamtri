use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;

use crate::mcp::client::McpClient;
use crate::mcp::tasks::{
    McpTask, McpTaskStatus, default_poll_interval_ms, parse_create_task_result, parse_task_value,
    task_state_from_mcp,
};
use crate::{CoreError, Result};

use super::gateway::GatewayEvent;

type EmitFn = Arc<dyn Fn(GatewayEvent) + Send + Sync>;

#[derive(Debug, Clone)]
struct TrackedTaskContext {
    server_id: String,
    origin_tool_call_id: Option<String>,
    title: Option<String>,
    subscribe_capable: bool,
}

#[derive(Debug)]
struct TrackedTask {
    context: TrackedTaskContext,
    last_mcp: McpTask,
    result: Option<Value>,
}

pub struct GatewayTaskTracker {
    tasks: Arc<Mutex<HashMap<String, TrackedTask>>>,
    poll_handles: Arc<Mutex<HashMap<String, AbortHandle>>>,
    suspended: Arc<Mutex<HashSet<String>>>,
    clients: Mutex<HashMap<String, Arc<McpClient>>>,
}

impl Default for GatewayTaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayTaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            poll_handles: Arc::new(Mutex::new(HashMap::new())),
            suspended: Arc::new(Mutex::new(HashSet::new())),
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub async fn register_client(&self, server_id: &str, client: Arc<McpClient>) {
        self.clients
            .lock()
            .await
            .insert(server_id.to_string(), client);
    }

    pub async fn unregister_client(&self, server_id: &str) {
        self.clients.lock().await.remove(server_id);
    }

    pub async fn client_for(&self, server_id: &str) -> Option<Arc<McpClient>> {
        self.clients.lock().await.get(server_id).cloned()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register_created_task(
        &self,
        server_id: &str,
        mcp_task: McpTask,
        origin_tool_call_id: Option<String>,
        title: Option<String>,
        subscribe_capable: bool,
        emit: EmitFn,
        client: Arc<McpClient>,
    ) {
        let state = task_state_from_mcp(
            server_id,
            &mcp_task,
            origin_tool_call_id.clone(),
            title.clone(),
            None,
        );
        emit(GatewayEvent::TaskStarted { state });
        self.tasks.lock().await.insert(
            mcp_task.task_id.clone(),
            TrackedTask {
                context: TrackedTaskContext {
                    server_id: server_id.to_string(),
                    origin_tool_call_id,
                    title,
                    subscribe_capable,
                },
                last_mcp: mcp_task.clone(),
                result: None,
            },
        );
        if !subscribe_capable {
            self.start_polling(mcp_task.task_id.clone(), emit, client)
                .await;
        }
    }

    pub async fn handle_status_notification(
        &self,
        server_id: &str,
        params: &Value,
        emit: EmitFn,
        client: Arc<McpClient>,
    ) {
        let Some(mcp_task) = parse_task_value(params) else {
            return;
        };
        self.apply_task_update(server_id, mcp_task, None, emit, client)
            .await;
    }

    pub async fn apply_task_update(
        &self,
        server_id: &str,
        mcp_task: McpTask,
        result: Option<Value>,
        emit: EmitFn,
        client: Arc<McpClient>,
    ) {
        let terminal = mcp_task.status.is_terminal();
        let context = {
            let mut tasks = self.tasks.lock().await;
            if let Some(entry) = tasks.get_mut(&mcp_task.task_id) {
                entry.last_mcp = mcp_task.clone();
                if result.is_some() {
                    entry.result = result.clone();
                }
                entry.context.clone()
            } else {
                TrackedTaskContext {
                    server_id: server_id.to_string(),
                    origin_tool_call_id: None,
                    title: None,
                    subscribe_capable: true,
                }
            }
        };
        let stored_result = self
            .tasks
            .lock()
            .await
            .get(&mcp_task.task_id)
            .and_then(|entry| entry.result.clone())
            .or(result);

        let state = task_state_from_mcp(
            &context.server_id,
            &mcp_task,
            context.origin_tool_call_id.clone(),
            context.title.clone(),
            stored_result.clone(),
        );
        if terminal {
            emit(GatewayEvent::TaskCompleted {
                state,
                result: stored_result,
            });
            self.stop_polling(&mcp_task.task_id).await;
            self.tasks.lock().await.remove(&mcp_task.task_id);
        } else {
            emit(GatewayEvent::TaskUpdated { state });
            if !context.subscribe_capable {
                self.ensure_polling(&mcp_task.task_id, emit, client).await;
            }
        }
    }

    pub async fn cancel_task(
        &self,
        task_id: &str,
        emit: EmitFn,
    ) -> Result<()> {
        let server_id = self
            .task_server_id(task_id)
            .await
            .ok_or_else(|| CoreError::Protocol(format!("unknown task: {task_id}")))?;
        let client = self
            .client_for(&server_id)
            .await
            .ok_or_else(|| CoreError::Protocol(format!("no client for task: {task_id}")))?;
        let raw = client.cancel_task(task_id).await?;
        let mcp_task = parse_task_value(&raw)
            .ok_or_else(|| CoreError::Protocol("invalid tasks/cancel response".to_string()))?;
        self.apply_task_update(&server_id, mcp_task, None, emit, client)
            .await;
        Ok(())
    }

    pub async fn task_server_id(&self, task_id: &str) -> Option<String> {
        self.tasks
            .lock()
            .await
            .get(task_id)
            .map(|task| task.context.server_id.clone())
    }

    pub async fn suspend_polling(&self, task_id: &str) {
        self.suspended.lock().await.insert(task_id.to_string());
        self.stop_polling(task_id).await;
    }

    pub async fn resume_polling(
        &self,
        task_id: &str,
        emit: EmitFn,
    ) -> Result<()> {
        self.suspended.lock().await.remove(task_id);
        let server_id = self
            .task_server_id(task_id)
            .await
            .ok_or_else(|| CoreError::Protocol(format!("unknown task: {task_id}")))?;
        let client = self
            .client_for(&server_id)
            .await
            .ok_or_else(|| CoreError::Protocol(format!("no client for task: {task_id}")))?;
        let raw = client.get_task(task_id).await?;
        let mcp_task = parse_task_value(&raw)
            .ok_or_else(|| CoreError::Protocol("invalid tasks/get response".to_string()))?;
        self.apply_task_update(&server_id, mcp_task, None, Arc::clone(&emit), Arc::clone(&client))
            .await;
        let subscribe_capable = self
            .tasks
            .lock()
            .await
            .get(task_id)
            .map(|task| task.context.subscribe_capable)
            .unwrap_or(false);
        if !subscribe_capable {
            self.ensure_polling(task_id, emit, client).await;
        }
        Ok(())
    }

    async fn ensure_polling(&self, task_id: &str, emit: EmitFn, client: Arc<McpClient>) {
        if self.suspended.lock().await.contains(task_id) {
            return;
        }
        if self.poll_handles.lock().await.contains_key(task_id) {
            return;
        }
        self.start_polling(task_id.to_string(), emit, client).await;
    }

    async fn start_polling(&self, task_id: String, emit: EmitFn, client: Arc<McpClient>) {
        let tasks = self.tasks.clone();
        let suspended = self.suspended.clone();
        let poll_handles = self.poll_handles.clone();
        let task_id_for_loop = task_id.clone();
        let handle = tokio::spawn(async move {
            loop {
                if suspended.lock().await.contains(&task_id_for_loop) {
                    break;
                }
                let snapshot = {
                    let tasks_guard = tasks.lock().await;
                    tasks_guard.get(&task_id_for_loop).map(|task| {
                        (
                            task.context.server_id.clone(),
                            task.context.origin_tool_call_id.clone(),
                            task.context.title.clone(),
                            task.last_mcp.clone(),
                            task.context.subscribe_capable,
                        )
                    })
                };
                let Some((server_id, origin_tool_call_id, title, last_mcp, subscribe_capable)) =
                    snapshot
                else {
                    break;
                };
                if subscribe_capable || last_mcp.status.is_terminal() {
                    break;
                }
                let interval = default_poll_interval_ms(&last_mcp);
                tokio::time::sleep(Duration::from_millis(interval)).await;
                if suspended.lock().await.contains(&task_id_for_loop) {
                    break;
                }
                let Ok(raw) = client.get_task(&task_id_for_loop).await else {
                    continue;
                };
                let Some(mcp_task) = parse_task_value(&raw) else {
                    continue;
                };
                let mut result = None;
                if mcp_task.status == McpTaskStatus::Completed
                    && let Ok(raw_result) = client.get_task_result(&task_id_for_loop).await
                {
                    result = Some(raw_result);
                }
                let terminal = mcp_task.status.is_terminal();
                {
                    let mut tasks_guard = tasks.lock().await;
                    if let Some(entry) = tasks_guard.get_mut(&task_id_for_loop) {
                        entry.last_mcp = mcp_task.clone();
                        if result.is_some() {
                            entry.result = result.clone();
                        }
                    }
                }
                let state = task_state_from_mcp(
                    &server_id,
                    &mcp_task,
                    origin_tool_call_id,
                    title,
                    result.clone(),
                );
                if terminal {
                    emit(GatewayEvent::TaskCompleted {
                        state,
                        result,
                    });
                    break;
                }
                emit(GatewayEvent::TaskUpdated { state });
            }
            poll_handles.lock().await.remove(&task_id_for_loop);
        });
        self.poll_handles
            .lock()
            .await
            .insert(task_id, handle.abort_handle());
    }

    async fn stop_polling(&self, task_id: &str) {
        if let Some(handle) = self.poll_handles.lock().await.remove(task_id) {
            handle.abort();
        }
    }
}

pub fn parse_task_from_tool_response(value: &Value) -> Option<McpTask> {
    parse_create_task_result(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_task_from_tool_response_reads_nested_task() {
        let value = json!({"task": {"taskId": "abc", "status": "working"}});
        let task = parse_task_from_tool_response(&value).expect("task");
        assert_eq!(task.task_id, "abc");
    }
}
