//! Direct Codex adapter via `codex app-server` NDJSON JSON-RPC on stdio.

mod events;
mod history;
mod session;

pub use history::parse_codex_session_file;

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;

use crate::harness::acp::AgentLaunchSpec;
use crate::harness::codex::session::{connect_and_initialize, effective_args, run_codex_session};
use crate::harness::{
    ConversationContext, HarnessAdapter, HarnessCapabilities, HarnessRun, ModelInfo, TurnInput,
};
use crate::rpc::dispatch::RpcConnection;
use crate::rpc::transport::stdio::StdioTransport;
use crate::{CoreError, Result};

const CODEX_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct CodexNativeAdapter {
    launch: AgentLaunchSpec,
}

impl CodexNativeAdapter {
    pub fn new(launch: AgentLaunchSpec) -> Self {
        Self { launch }
    }
}

#[async_trait]
impl HarnessAdapter for CodexNativeAdapter {
    fn id(&self) -> &str {
        &self.launch.id
    }

    fn display_name(&self) -> &str {
        &self.launch.display_name
    }

    fn agent_launch_spec(&self) -> Option<AgentLaunchSpec> {
        Some(self.launch.clone())
    }

    fn capabilities(&self) -> HarnessCapabilities {
        HarnessCapabilities {
            streaming: true,
            tools: true,
            permissions: true,
            thinking: true,
            native_tools: true,
        }
    }

    async fn run(&self, ctx: ConversationContext, turn: TurnInput) -> Result<HarnessRun> {
        let args = effective_args(&self.launch.args);
        let cwd = session::spawn_cwd(&ctx);
        let transport = StdioTransport::spawn_with_cwd(
            &self.launch.command,
            &args,
            &self.launch.env,
            Some(&cwd),
        )
        .await?;
        let (rpc, inbound) = RpcConnection::start(Box::new(transport));

        let (event_tx, event_rx) = tokio::sync::mpsc::channel(128);
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(32);
        let harness_id = self.launch.id.clone();
        tokio::spawn(run_codex_session(
            rpc,
            inbound,
            command_rx,
            event_tx,
            ctx,
            turn,
            harness_id,
        ));

        Ok(HarnessRun {
            events: event_rx,
            control: crate::harness::RunControl::new(command_tx),
        })
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        list_models(&self.launch, None).await
    }
}

async fn list_models(launch: &AgentLaunchSpec, cwd: Option<&Path>) -> Result<Vec<ModelInfo>> {
    let (rpc, _inbound) = connect_and_initialize(launch, cwd).await?;
    let response = rpc
        .request(
            "model/list",
            Some(serde_json::json!({})),
            CODEX_REQUEST_TIMEOUT,
        )
        .await?;
    let _ = rpc.close().await;

    let models = response
        .get("data")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    if models.is_empty() {
        return Err(CoreError::Protocol(
            "Codex app-server returned no models".to_string(),
        ));
    }

    Ok(models
        .into_iter()
        .filter_map(|model| {
            let id = model.get("id")?.as_str()?.to_string();
            let display_name = model
                .get("displayName")
                .or_else(|| model.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&id)
                .to_string();
            Some(ModelInfo { id, display_name })
        })
        .collect())
}
