//! Direct Codex adapter shell via codex-app-server. ACP cannot map Codex session
//! fidelity; this adapter owns the native wire instead.
//!
//! V1 wires the roster seam and capability flags; the app-server transport lands
//! in the next iteration.

use async_trait::async_trait;

use crate::harness::acp::AgentLaunchSpec;
use crate::harness::{
    ConversationContext, HarnessAdapter, HarnessCapabilities, HarnessRun, ModelInfo, TurnInput,
};
use crate::{CoreError, Result};

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

    fn capabilities(&self) -> HarnessCapabilities {
        HarnessCapabilities {
            streaming: true,
            tools: true,
            permissions: false,
            thinking: false,
            native_tools: true,
        }
    }

    async fn run(&self, _ctx: ConversationContext, _turn: TurnInput) -> Result<HarnessRun> {
        Err(CoreError::Protocol(
            "CodexNativeAdapter transport not wired yet; use adapter=acp in config.json for now"
                .to_string(),
        ))
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
}
