use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::{Duration, timeout};

use crate::rpc::jsonrpc::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::rpc::transport::Transport;
use crate::{CoreError, Result};

pub struct StdioTransport {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(command: &str, args: &[String], env: &[(String, String)]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        preserve_env(&mut cmd, "PATH");
        preserve_env(&mut cmd, "HOME");
        preserve_env(&mut cmd, "TMPDIR");
        preserve_env(&mut cmd, "LANG");
        for (key, value) in env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or(CoreError::TransportClosed)?;
        let stdout = child.stdout.take().ok_or(CoreError::TransportClosed)?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) => {
                            tracing::debug!(target: "tamtri_core::rpc::stderr", "{line}");
                        }
                        Ok(None) => break,
                        Err(err) => {
                            tracing::debug!(target: "tamtri_core::rpc::stderr", "stderr read failed: {err}");
                            break;
                        }
                    }
                }
            });
        }

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    async fn send_json<T: serde::Serialize>(&mut self, message: &T) -> Result<()> {
        let stdin = self.stdin.as_mut().ok_or(CoreError::TransportClosed)?;
        let mut line = serde_json::to_string(message)?;
        line.push('\n');
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Transport for StdioTransport {
    async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
        self.send_json(req).await
    }

    async fn send_notification(&mut self, note: &JsonRpcNotification) -> Result<()> {
        self.send_json(note).await
    }

    async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()> {
        self.send_json(resp).await
    }

    async fn recv(&mut self) -> Result<IncomingMessage> {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).await?;
        if read == 0 {
            return Err(CoreError::TransportClosed);
        }
        IncomingMessage::from_line(line.trim_end_matches('\n'))
    }

    async fn close(&mut self) -> Result<()> {
        drop(self.stdin.take());
        match timeout(Duration::from_secs(2), self.child.wait()).await {
            Ok(status) => {
                let _status = status?;
            }
            Err(_) => {
                self.child.kill().await?;
                let _status = self.child.wait().await?;
            }
        }
        Ok(())
    }
}

fn preserve_env(cmd: &mut Command, key: &str) {
    if let Ok(value) = std::env::var(key) {
        cmd.env(key, value);
    }
}
