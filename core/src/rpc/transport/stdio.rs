use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::{Duration, timeout};

use crate::harness::spawn_env::preserve_spawn_env_tokio;
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
        Self::spawn_with_cwd(command, args, env, None).await
    }

    pub async fn spawn_with_cwd(
        command: &str,
        args: &[String],
        env: &[(String, String)],
        cwd: Option<&Path>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = cwd {
            std::fs::create_dir_all(cwd)?;
            cmd.current_dir(cwd);
        }
        preserve_spawn_env_tokio(&mut cmd);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::jsonrpc::IncomingMessage;

    #[tokio::test]
    async fn spawn_scrubs_parent_only_environment_variables() {
        // SAFETY: test-only env mutation; no concurrent tests rely on this variable.
        unsafe { std::env::set_var("TAMTRI_STDIO_ENV_SCRUB_TEST", "must-not-leak") };
        let script = r#"printf '%s\n' "{\"jsonrpc\":\"2.0\",\"method\":\"env_probe\",\"params\":{\"secret\":\"${TAMTRI_STDIO_ENV_SCRUB_TEST:-}\",\"has_path\":\"${PATH:+yes}\"}}""#;
        let mut transport =
            StdioTransport::spawn("/bin/sh", &[String::from("-c"), script.to_string()], &[])
                .await
                .expect("spawn probe shell");
        let message = transport.recv().await.expect("env probe line");
        let params = match message {
            IncomingMessage::Notification(note) => {
                note.params.expect("env_probe params should be present")
            }
            other => panic!("expected env_probe notification, got {other:?}"),
        };
        assert_eq!(params["secret"], "");
        assert_eq!(params["has_path"], "yes");
        transport.close().await.expect("close probe shell");
    }
}
