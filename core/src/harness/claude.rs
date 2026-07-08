//! Direct Claude adapter shell. Speaks the native Claude Code interface rather
//! than ACP, for fidelity ACP cannot carry (thinking, tool diffs, session files).
//!
//! V1 wires the roster seam and capability flags; the subprocess/SDK transport
//! lands in the next iteration.

use async_trait::async_trait;

use crate::harness::acp::AgentLaunchSpec;
use crate::harness::{
    ConversationContext, HarnessAdapter, HarnessCapabilities, HarnessRun, ModelInfo, TurnInput,
};
use crate::{CoreError, Result};

pub struct ClaudeNativeAdapter {
    launch: AgentLaunchSpec,
}

impl ClaudeNativeAdapter {
    pub fn new(launch: AgentLaunchSpec) -> Self {
        Self { launch }
    }
}

#[async_trait]
impl HarnessAdapter for ClaudeNativeAdapter {
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
            permissions: true,
            thinking: true,
            native_tools: true,
        }
    }

    async fn run(&self, _ctx: ConversationContext, _turn: TurnInput) -> Result<HarnessRun> {
        Err(CoreError::Protocol(
            "ClaudeNativeAdapter transport not wired yet; use adapter=acp in config.json for now"
                .to_string(),
        ))
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
}
