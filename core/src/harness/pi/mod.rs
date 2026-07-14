//! Direct Pi adapter via `pi --mode rpc` JSON lines on stdio.

mod events;
mod rpc;
mod session;

use async_trait::async_trait;

use crate::Result;
use crate::harness::acp::AgentLaunchSpec;
use crate::harness::pi::session::{connect_for_models, run_pi_session};
use crate::harness::{
    ConversationContext, HarnessAdapter, HarnessCapabilities, HarnessRun, ModelInfo, TurnInput,
};

pub struct PiNativeAdapter {
    launch: AgentLaunchSpec,
}

impl PiNativeAdapter {
    pub fn new(launch: AgentLaunchSpec) -> Self {
        Self { launch }
    }
}

#[async_trait]
impl HarnessAdapter for PiNativeAdapter {
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
            native_tools: false,
            runtime_model_switch: true,
        }
    }

    async fn run(&self, ctx: ConversationContext, turn: TurnInput) -> Result<HarnessRun> {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(128);
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(32);
        let launch = self.launch.clone();
        let harness_id = self.launch.id.clone();
        tokio::spawn(run_pi_session(
            launch, ctx, turn, command_rx, event_tx, harness_id,
        ));

        Ok(HarnessRun {
            events: event_rx,
            control: crate::harness::RunControl::new(command_tx),
        })
    }

    async fn available_models(&self) -> Result<Vec<ModelInfo>> {
        connect_for_models(&self.launch).await
    }
}
