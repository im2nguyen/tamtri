//! Direct Claude Code adapter via `claude --print --output-format stream-json`.

mod events;
mod history;
mod session;

pub use history::parse_claude_session_file;

use async_trait::async_trait;

use crate::Result;
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::claude::session::run_claude_session;
use crate::harness::{
    ConversationContext, HarnessAdapter, HarnessCapabilities, HarnessRun, ModelInfo, TurnInput,
};

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
            runtime_model_switch: true,
        }
    }

    async fn run(&self, ctx: ConversationContext, turn: TurnInput) -> Result<HarnessRun> {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(128);
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(32);
        let launch = self.launch.clone();
        let harness_id = self.launch.id.clone();
        tokio::spawn(run_claude_session(
            launch, ctx, turn, command_rx, event_tx, harness_id,
        ));

        Ok(HarnessRun {
            events: event_rx,
            control: crate::harness::RunControl::new(command_tx),
        })
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }
}
