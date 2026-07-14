//! Build a [`HarnessAdapter`] from a roster [`AgentLaunchSpec`]. The registry is
//! the seam where heterogeneous providers (ACP fallback, direct Claude, direct
//! Codex) plug in behind one trait.

use std::sync::Arc;

use crate::harness::HarnessAdapter;
use crate::harness::acp::{AcpAdapter, AdapterKind, AgentLaunchSpec};
use crate::harness::claude::ClaudeNativeAdapter;
use crate::harness::codex::CodexNativeAdapter;
use crate::harness::opencode::OpenCodeNativeAdapter;
use crate::harness::pi::PiNativeAdapter;

pub fn build_adapter(spec: &AgentLaunchSpec) -> Arc<dyn HarnessAdapter> {
    match spec.adapter {
        AdapterKind::Acp => Arc::new(AcpAdapter::new(spec.clone())),
        AdapterKind::ClaudeNative => Arc::new(ClaudeNativeAdapter::new(spec.clone())),
        AdapterKind::CodexNative => Arc::new(CodexNativeAdapter::new(spec.clone())),
        AdapterKind::OpenCodeNative => Arc::new(OpenCodeNativeAdapter::new(spec.clone())),
        AdapterKind::PiNative => Arc::new(PiNativeAdapter::new(spec.clone())),
    }
}
