# Milestone 6: Elicitation + Remote Auth

**Status: Complete.** Form and URL elicitation, OAuth 2.1 + PKCE, keychain tokens, hermetic + Swift policy tests.

Sixth build session. Downstream MCP servers can now ask the user a follow-up question through tamtri, and remote HTTP servers can authenticate without the user touching a terminal. The agent still sees an ordinary tool call: tamtri pauses the downstream call, renders the prompt or browser handoff, collects the result, and resumes the call.

The security rule is simple: never route secrets through elicitation form mode. Form mode is for structured non-secret answers. Secrets and account authorization use URL/OAuth flows to trusted domains, with values stored in the macOS keychain and referenced from the vault.

## Definition of done

- The gateway advertises elicitation support to downstream servers and handles server-initiated elicitation requests while a tool call is pending.
- Form-mode elicitation renders as a native SwiftUI card, validates against the server-provided schema subset, and returns accept/decline/cancel to the downstream server.
- URL-mode elicitation renders a consent sheet/card showing the exact destination, then hands off to the browser only after explicit approval.
- Elicitation cards nest under the originating tool call via `origin_tool_call_id` and persist as `ElicitationRequest` plus `ElicitationResponse` blocks.
- The agent receives the completed downstream tool result and does not need to know elicitation happened.
- OAuth 2.1 works for remote streamable HTTP servers: authorization code + PKCE, tokens in keychain, silent refresh, and re-auth prompt on refresh failure.
- `events.jsonl` records elicitation request/response receipts and OAuth lifecycle receipts without secrets.
- The shell exposes keychain-backed credential status for remote servers. The vault stores references only.
- Hermetic tests cover form mode, URL mode, OAuth success/refresh/failure, persistence, and secret redaction. `cargo test` and `cargo clippy` are clean for core work.
- `/docs/mcp-client.md`, `/docs/events-format.md`, and any auth docs describe elicitation routing, OAuth storage, and redaction behavior.

## Architecture note: pause the downstream call, not the app

Elicitation is a server-initiated request from a downstream MCP server. It can arrive while the gateway is waiting for a `tools/call` result. The gateway should suspend only that downstream request path and surface a UI event. Other conversations and unrelated tool calls continue.

Suggested event shape:

```rust
pub enum GatewayEvent {
    ElicitationRequested {
        origin_tool_call_id: Option<String>,
        server_id: String,
        request_id: String,
        mode: ElicitationMode,
        message: String,
        schema: Option<Value>,
        url: Option<String>,
    },
    ElicitationResolved {
        server_id: String,
        request_id: String,
        action: ElicitationAction,
    },
}
```

The transcript gets the compact, user-visible version. The audit log gets the full server id, request id, schema, routing, timestamps, and action, minus any secret values.

## Task 1: Elicitation protocol and gateway routing

Extend MCP protocol support for elicitation requests from downstream servers.

Behavior:

- Advertise elicitation support only to downstream servers, not to the upstream agent-facing gateway unless the gateway actually handles it there.
- When an elicitation request arrives, correlate it to the active downstream `tools/call` and the upstream tool card if available.
- Emit `GatewayEvent::ElicitationRequested`.
- Wait for the UI response without blocking unrelated gateway traffic.
- Respond to the downstream server with accept, decline, or cancel using the protocol's expected result shape.
- Apply a reasonable timeout or cancellation path if the user dismisses the run.
- If the app quits mid-elicitation, cancel/decline the downstream request and record the outcome.

Tests: elicitation while a tool call is pending, two unrelated tool calls where only one elicits, user accept, user decline, user cancel, run cancel while eliciting, and unsupported malformed requests returning protocol errors.

## Task 2: Persist elicitation blocks

Map gateway events into the conversation model:

- `ElicitationRequested` -> `ContentBlock::ElicitationRequest`
- user answer -> `ContentBlock::ElicitationResponse`

Rules:

- The visible transcript stores the message, mode, non-secret schema, URL host/path as appropriate, and the user action.
- Do not store entered secret-looking fields in the transcript or audit log.
- Nest the live card under the originating tool call when `origin_tool_call_id` exists.
- Reload from `messages.jsonl` must show the historical request and whether it was accepted, declined, or cancelled.

Tests: persisted request/response golden files, reload redraws historical elicitation, compact transcript form omits secret-looking values, and audit receipts include routing without secrets.

## Task 3: Native form renderer

Build a SwiftUI form card for structured elicitation.

Schema subset for V1:

- object root with properties.
- string, number, integer, boolean.
- enum/select for string enums.
- arrays of primitive values only if the schema is simple enough to render clearly.
- required fields.
- min/max length and numeric min/max where easy.
- descriptions/help text from the schema.

If the schema is too complex, show a graceful unsupported-schema card with decline/cancel actions. Do not guess.

Secret guard:

- If a field name, title, description, or format looks like a password, token, API key, private key, or secret, do not render it in form mode.
- Show a "use trusted browser flow" style error and decline the request unless the server provided URL mode.

Tests: basic form, validation errors, enum select, required fields, unsupported nested schema, secret-looking field blocked, keyboard navigation, and VoiceOver labels.

## Task 4: URL-mode elicitation

URL mode is a trusted-domain handoff, not an embedded browser.

Behavior:

- Show the exact destination origin and full URL before opening.
- Reject non-HTTPS URLs except loopback callbacks used by OAuth.
- Reject userinfo URLs, suspicious redirects, and mismatched hosts.
- Open the system browser only after explicit user approval.
- Return the appropriate accept/decline/cancel result to the downstream server.
- Record the handoff in `events.jsonl` with origin and server id, not query secrets.

Use the same consent copy and URL validation machinery that OAuth uses in Task 5. This is why URL elicitation and auth live in one milestone.

Tests: HTTPS allowed after consent, HTTP rejected except loopback, userinfo URL rejected, query string redacted in logs, user decline, browser handoff result, and downstream response shape.

## Task 5: OAuth 2.1 for remote MCP servers

Complete the remote-server credential story.

Requirements:

- Authorization code flow with PKCE.
- Discover authorization metadata from the server when available; otherwise support explicit config in the gateway registry.
- Browser handoff with exact host consent.
- Loopback callback or app callback handled by the shell, then passed to core.
- Access and refresh tokens stored in the macOS keychain.
- Vault config stores token references and auth metadata only.
- Silent refresh before expiry.
- Re-auth prompt when refresh fails.
- No token values in `messages.jsonl`, `events.jsonl`, logs, crash reports, or diagnostics.

Suggested registry extension:

```rust
pub struct OAuthConfig {
    pub issuer: Option<String>,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub token_ref: String,
}
```

Tests: PKCE verifier/challenge generation, auth URL construction, callback state validation, token exchange through a mock server, refresh through a mock server, refresh failure triggers re-auth state, and redaction in logs/events.

## Task 6: Keychain and FFI bridge

Keep platform-specific storage in the shell.

Core surface:

- Request a credential by reference.
- Receive a resolved secret in memory.
- Ask the shell to store/update/delete a token by reference.
- Report credential status without exposing values.

Shell responsibilities:

- Use macOS keychain for static secrets and OAuth tokens.
- Bind entries to stable tamtri service/account names.
- Handle missing, denied, or corrupt keychain entries gracefully.
- Avoid placing secret values in SwiftUI state that gets logged or snapshotted.

Tests: mock credential resolver in core tests, keychain adapter unit/smoke tests where possible, missing credential path, denied credential path, and secret values never crossing into debug descriptions.

## Task 7: UI integration

Add the visible pieces:

- Elicitation form cards under the relevant tool call.
- URL consent sheet/card with exact host and server attribution.
- OAuth connection flow in gateway server settings.
- Credential status badges: missing, connected, expired, needs re-auth.
- Non-blocking transcript behavior while a card waits for input.

The card contract mirrors permission cards: who is asking, what they need, what happens next, and equally visible decline/cancel paths.

Tests: UI snapshot/state tests for form, URL, OAuth needed, OAuth connected, and re-auth required.

## Task 8: Fixtures and tests

Extend fixtures:

- Downstream MCP stdio fixture that elicits during `tools/call`.
- `fixtures/twenty-questions-mcp`: 20 Questions game with form-mode elicitation for manual and hermetic gateway tests (see [docs/testing/twenty-questions.md](testing/twenty-questions.md)).
- Downstream HTTP fixture with OAuth-protected endpoints.
- Mock authorization server for PKCE and refresh.
- Mock ACP agent path that calls a gateway tool which elicits.

Enumerated tests:

1. `gateway_elicitation_form_accept_round_trip` - downstream server receives accepted data and tool call completes.
2. `gateway_elicitation_decline_round_trip`.
3. `gateway_elicitation_cancel_on_run_cancel`.
4. `elicitation_nested_under_origin_tool_call`.
5. `elicitation_persists_request_and_response_blocks`.
6. `elicitation_secret_field_rejected`.
7. `elicitation_complex_schema_graceful_fallback`.
8. `url_elicitation_requires_https`.
9. `url_elicitation_redacts_query_in_events`.
10. `oauth_pkce_flow_stores_token_reference_only`.
11. `oauth_refresh_success_updates_keychain`.
12. `oauth_refresh_failure_marks_reauth_required`.
13. `remote_http_server_uses_oauth_header_without_logging_value`.
14. `agent_receives_tool_result_after_elicitation`.
15. `reload_shows_historical_elicitation`.

Keep any real OAuth provider test manual and `#[ignore]`.

## Out of scope this milestone

Do not build MCP Apps, Tasks, or Roots (M7). Do not build a password manager. Do not render arbitrary login pages inside tamtri. Do not add team/cloud auth. Do not add sampling. Do not add full onboarding polish beyond the gateway settings needed for auth.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-4.md, docs/milestone-6.md, and docs/tamtri-decisions.md sections 9, 15, and 17. Implement Milestone 6. Start with gateway elicitation routing and persistence, then stop and show me the request/response mapping plus the secret-field guard before building OAuth. Secrets must never enter form mode, transcripts, events, or logs.
