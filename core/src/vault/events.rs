use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub ts: DateTime<Utc>,
    pub kind: EventKind,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(kind: EventKind, payload: serde_json::Value) -> Self {
        Self {
            ts: Utc::now(),
            kind,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    TurnStarted,
    TurnEnded,
    PermissionRequested,
    PermissionResolved,
    ToolCallStarted,
    ToolCallCompleted,
    ArtifactSnapshotted,
    ArtifactNavigationBlocked,
    GatewayServerConnected,
    GatewayToolRouted,
    GatewayProgress,
    GatewayLog,
    GatewayCancellation,
    GatewayCredentialInjected,
    GatewayDownstreamError,
    HarnessSpawned,
    HarnessExited,
    Error,
}

pub fn event_to_line(event: &Event) -> Result<String> {
    reject_secret_values(&event.payload)?;
    let line = serde_json::to_string(event)?;
    if line.contains('\n') {
        return Err(CoreError::MalformedVault(
            "event serialization produced a newline".to_string(),
        ));
    }
    Ok(line)
}

pub fn event_from_line(line: &str) -> Result<Event> {
    let event: Event = serde_json::from_str(line)?;
    reject_secret_values(&event.payload)?;
    Ok(event)
}

fn reject_secret_values(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let key = key.to_ascii_lowercase();
                if key.contains("secret")
                    || key.contains("token")
                    || key.contains("password")
                    || key.contains("api_key")
                {
                    return Err(CoreError::MalformedVault(
                        "event payload must not contain secret-like fields".to_string(),
                    ));
                }
                reject_secret_values(value)?;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                reject_secret_values(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}
