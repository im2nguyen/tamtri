use serde_json::json;
use tamtri_core::mcp::elicitation::sanitize_transcript_data;
use tamtri_core::vault::events::{event_to_line, Event, EventKind};

#[test]
fn events_jsonl_rejects_secret_like_payload_fields() {
    let event = Event::new(
        EventKind::GatewayCredentialInjected,
        json!({
            "server_id": "remote",
            "credential_ref": "api_key",
            "api_key": "must-not-persist"
        }),
    );
    assert!(event_to_line(&event).is_err());
}

#[test]
fn elicitation_transcript_data_redacts_secret_keys() {
    let sanitized = sanitize_transcript_data(&json!({
        "name": "tamtri",
        "access_token": "secret-value"
    }));
    assert_eq!(sanitized["name"], "tamtri");
    assert_eq!(sanitized["access_token"], "[redacted]");
}
