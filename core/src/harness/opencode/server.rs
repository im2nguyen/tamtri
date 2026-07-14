use std::net::TcpListener;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout};

use crate::harness::acp::AgentLaunchSpec;
use crate::harness::spawn_env::preserve_spawn_env_tokio;
use crate::{CoreError, Result};

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(200);

pub struct OpenCodeServer {
    pub base_url: String,
    child: Child,
}

impl OpenCodeServer {
    pub async fn spawn(launch: &AgentLaunchSpec) -> Result<Self> {
        let port = allocate_port()?;
        let hostname = "127.0.0.1";
        let base_url = format!("http://{hostname}:{port}");
        let mut args = launch.args.clone();
        if !has_serve_subcommand(&args) {
            args.push("serve".to_string());
        }
        if !has_port_flag(&args) {
            args.extend(["--port".to_string(), port.to_string()]);
        }
        if !has_hostname_flag(&args) {
            args.extend(["--hostname".to_string(), hostname.to_string()]);
        }

        let mut cmd = Command::new(&launch.command);
        cmd.args(&args)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        preserve_spawn_env_tokio(&mut cmd);
        for (key, value) in &launch.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn().map_err(CoreError::from)?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "tamtri_core::opencode::stderr", "{line}");
                }
            });
        }

        wait_for_health(&base_url, SERVER_START_TIMEOUT).await?;
        Ok(Self { base_url, child })
    }

    pub async fn shutdown(mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }
}

async fn wait_for_health(base_url: &str, max_wait: Duration) -> Result<()> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| CoreError::Protocol(format!("OpenCode HTTP client failed: {err}")))?;
    let health_url = format!("{base_url}/global/health");
    let started = std::time::Instant::now();
    while started.elapsed() < max_wait {
        match timeout(Duration::from_secs(2), client.get(&health_url).send()).await {
            Ok(Ok(response)) if response.status().is_success() => return Ok(()),
            _ => sleep(HEALTH_POLL_INTERVAL).await,
        }
    }
    Err(CoreError::Protocol(
        "OpenCode server failed to become healthy".to_string(),
    ))
}

fn allocate_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|err| CoreError::Protocol(format!("failed to allocate port: {err}")))?;
    let port = listener
        .local_addr()
        .map_err(|err| CoreError::Protocol(format!("failed to read allocated port: {err}")))?
        .port();
    Ok(port)
}

fn has_serve_subcommand(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "serve")
}

fn has_port_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--port" || arg.starts_with("--port="))
}

fn has_hostname_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--hostname" || arg.starts_with("--hostname="))
}

pub fn absolute_directory(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_port_returns_ephemeral_port() {
        let port = allocate_port().expect("port");
        assert!(port > 0);
    }
}
