//! Durable credential storage owned by the daemon.
//!
//! Gateway secrets and OAuth tokens persist across daemon restarts. Values never
//! appear in logs, `events.jsonl`, or the legible vault transcript. The store
//! implements [`crate::mcp::gateway::CredentialResolver`] for the MCP gateway.

mod sealed;

#[cfg(target_os = "macos")]
mod keychain;

pub use sealed::DurableCredentials;
