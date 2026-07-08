pub mod app;
pub mod artifact;
pub mod config;
pub mod diagnostics;
pub mod conversation;
pub mod daemon;
pub mod debug_log;
pub mod error;
pub mod harness;
pub mod mcp;
pub mod protocol;
pub mod rpc;
pub mod search;
pub mod vault;

pub use app::{ConversationDto, ConversationObserver, ConversationSummaryDto, TamtriCore, UiEvent};
pub use error::{CoreError, Result};

uniffi::setup_scaffolding!();
