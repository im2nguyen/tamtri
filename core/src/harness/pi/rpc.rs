use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::timeout;

use crate::{CoreError, Result};

#[derive(Clone)]
pub struct PiRpcHandle {
    state: Arc<Mutex<PiRpcState>>,
}

struct PiRpcState {
    stdin: ChildStdin,
    pending: HashMap<String, oneshot::Sender<Result<Value>>>,
    next_request_id: u64,
}

impl PiRpcHandle {
    pub fn start(
        stdin: ChildStdin,
        stdout: ChildStdout,
        event_tx: mpsc::Sender<Value>,
    ) -> (Self, tokio::task::JoinHandle<Result<()>>) {
        let handle = Self {
            state: Arc::new(Mutex::new(PiRpcState {
                stdin,
                pending: HashMap::new(),
                next_request_id: 1,
            })),
        };
        let reader_state = Arc::clone(&handle.state);
        let reader =
            tokio::spawn(async move { read_stdout_loop(stdout, reader_state, event_tx).await });
        (handle, reader)
    }

    pub async fn request(&self, mut command: Value, request_timeout: Duration) -> Result<Value> {
        let id = {
            let mut state = self.state.lock().await;
            let id = format!("req_{}", state.next_request_id);
            state.next_request_id += 1;
            if let Some(object) = command.as_object_mut() {
                object.insert("id".to_string(), Value::String(id.clone()));
            }
            id
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut state = self.state.lock().await;
            state.pending.insert(id.clone(), tx);
            state.write_line(&command).await?;
        }

        match timeout(request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CoreError::TransportClosed),
            Err(_) => {
                self.state.lock().await.pending.remove(&id);
                Err(CoreError::Protocol(format!(
                    "Pi RPC request timed out for {}",
                    command
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                )))
            }
        }
    }

    pub async fn write_line(&self, value: &Value) -> Result<()> {
        self.state.lock().await.write_line(value).await
    }
}

impl PiRpcState {
    async fn write_line(&mut self, value: &Value) -> Result<()> {
        let mut line = serde_json::to_string(value)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    fn handle_response(&mut self, response: &Value) {
        let Some(id) = response.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some(tx) = self.pending.remove(id) else {
            return;
        };
        let success = response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let result = if success {
            Ok(response.get("data").cloned().unwrap_or(Value::Null))
        } else {
            Err(CoreError::Protocol(
                response
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("Pi RPC command failed")
                    .to_string(),
            ))
        };
        let _ = tx.send(result);
    }

    fn fail_pending(&mut self, err: CoreError) {
        let message = err.to_string();
        for (_, tx) in self.pending.drain() {
            let _ = tx.send(Err(CoreError::Protocol(message.clone())));
        }
    }
}

async fn read_stdout_loop(
    stdout: ChildStdout,
    state: Arc<Mutex<PiRpcState>>,
    event_tx: mpsc::Sender<Value>,
) -> Result<()> {
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if parsed.get("type").and_then(Value::as_str) == Some("response") {
            state.lock().await.handle_response(&parsed);
            continue;
        }
        if event_tx.send(parsed).await.is_err() {
            break;
        }
    }
    state.lock().await.fail_pending(CoreError::TransportClosed);
    Err(CoreError::TransportClosed)
}

pub async fn shutdown_child(mut child: Child) {
    let _ = child.start_kill();
    let _ = child.wait().await;
}
