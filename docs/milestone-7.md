# Milestone 7: Apps + Tasks + Roots

**Status: feature complete (branch `reconciliation/milestones-complete`).** Apps, Tasks, and Roots are wired end-to-end with RC capability gates, fixtures, settings badges, accessibility fallbacks, composer attach-root → roots picker, task subscribe fixture + test, app redirect-block coverage, roots_listed audit receipts, live task nesting under origin tool calls, app bridge allow-for-conversation consent, persisted app state rehydration on webview reload, and docs. Sampling remains declined by design.

Seventh build session. The remaining rich MCP primitives come online. Apps reuse the M5 webview host or React renderer island with a stricter declared-origin policy and a consent-gated UI-to-host bridge. Tasks turn long-running server work into durable task cards. Roots let users attach filesystem or knowledge roots per conversation and expose them through the gateway.

This milestone completes the planned MCP coverage for V1, except sampling. Sampling stays declined because tamtri is not the model; the model lives in the harness.

## Definition of done

- MCP Apps returned by downstream servers render inline from pre-declared templates, with network access limited to declared origins and no access to the host except through the audited bridge.
- UI-initiated App actions use the same consent/audit path as direct tool calls. The user sees which server/App is asking and what action will run.
- Tasks show as live task cards, poll or subscribe for status as supported, allow cancellation, support mid-task input when the server asks, and persist final state as `TaskRef` blocks.
- Roots can be attached per conversation from the shell, stored legibly as refs in `meta.json`, backed by macOS security-scoped bookmarks, and exposed to downstream servers through the gateway.
- 2026-07-28 RC behaviors for Apps/Tasks as extensions are gated behind capability checks, while MCP 2025-11-25 servers continue to work.
- The gateway continues to proxy tools/resources/prompts/elicitation from M4/M6 without regressions.
- Hermetic fixtures cover Apps, Tasks, Roots, RC capability gates, and consent for UI-initiated actions. `cargo test` and `cargo clippy` are clean for core work.
- Docs describe App sandbox origins, task persistence, roots security, and the explicit sampling-declined posture.

## Architecture note: reuse infrastructure, loosen only by capability

M5 built the no-network artifact webview and may have introduced the React/TypeScript renderer island. Apps need a related but different sandbox:

```text
Artifacts: no network, no host bridge
Apps: declared origins only, host bridge only through consent-gated JSON-RPC
```

Do not build a second renderer from scratch. Factor the webview host around a policy object:

```swift
enum WebContentPolicy {
    case artifactNoNetwork
    case app(allowedOrigins: [Origin], appId: String, serverId: String)
}
```

The bridge is disabled for artifacts. For Apps, every host action routes through the same gateway consent/audit path as tool calls.

If the React renderer hosts both transcript cards and App frames, keep two bridges: the trusted app-renderer bridge for view intents, and the MCP App bridge for server-provided App code. Do not let MCP App code call the trusted renderer bridge.

## Task 1: MCP capability gates and extension negotiation

Extend protocol support for Apps, Tasks, and Roots while supporting both the stable spec generation and RC extension-style behavior.

Rules:

- Detect server capabilities at initialize and per feature list call.
- Advertise only what tamtri supports for the relevant side of the gateway.
- Decline sampling honestly by omitting the capability or returning the documented unsupported response.
- Do not let an RC-only field break 2025-11-25 parsing. Keep unknown payloads as raw JSON where the shape is still changing.
- Add a capability report to the settings/debug surface so it is clear why a feature is unavailable.

Tests: stable server without extensions, RC server with Apps/Tasks extensions, unknown extension ignored, sampling request declined cleanly, and no feature advertised before support is wired.

## Task 2: MCP Apps resource model

Add core types for App resources returned by downstream servers.

Suggested shape:

```rust
pub struct AppTemplate {
    pub template_ref: String,
    pub server_id: String,
    pub html: String,
    pub allowed_origins: Vec<Origin>,
    pub metadata: serde_json::Value,
}

pub struct AppInstance {
    pub uri: String,
    pub template_ref: String,
    pub server_id: String,
    pub state: serde_json::Value,
    pub origin_tool_call_id: Option<String>,
}
```

Behavior:

- Load only templates declared by the server through the gateway.
- Validate allowed origins before any webview gets network access.
- Persist an `AppResource` block with URI, template ref, and state.
- Reloading from `messages.jsonl` shows the App historical state even if the server is offline. If interactivity requires the server and it is unavailable, show a disabled state rather than failing the transcript.

Tests: template registration, undeclared template rejected, bad origin rejected, app resource block persisted, reload with server offline, and origin tool call nesting.

## Task 3: App webview and UI-to-host bridge

Enable the App variant of the M5 webview host or renderer island.

Bridge requirements:

- App JavaScript talks to the host through a narrow JSON-RPC channel.
- Every App-initiated tool call, resource read, or state mutation goes through a consent card before the gateway executes it.
- Consent card shows server id, app/template identity, action name, and readable argument summary.
- Deny is as visible as allow.
- Audit receipts include app id/template, server id, action, arguments summary, and resolution. No secrets.
- The bridge cannot call arbitrary shell APIs.
- The MCP App bridge is isolated from any trusted React renderer bridge.

Network requirements:

- Only declared origins are reachable.
- Redirects to undeclared origins are blocked.
- Cookies/storage are scoped to the App webview policy and do not leak to artifact views.

Tests: app loads declared template, undeclared origin blocked, bridge action asks consent, deny blocks action, allow routes through gateway, audit receipt written, reload restores state, and artifact webviews still have no bridge.

## Task 4: Tasks protocol and state machine

Implement MCP Tasks as durable, non-blocking work.

Suggested state:

```rust
pub struct TaskState {
    pub task_id: String,
    pub server_id: String,
    pub status: TaskStatus,
    pub title: Option<String>,
    pub progress: Option<TaskProgress>,
    pub result: Option<serde_json::Value>,
    pub origin_tool_call_id: Option<String>,
}
```

Behavior:

- Start or observe tasks returned by downstream servers.
- Poll status or subscribe to updates depending on server capability.
- Render live task cards.
- Allow cancellation when the server supports it.
- Route mid-task input through the elicitation/input path from M6.
- Persist final state as `TaskRef`; keep detailed updates in `events.jsonl`.
- Tasks must not block the whole conversation. Users can continue reading and interacting while a task runs.

Tests: task started, progress update, completion result, failure, cancellation, app background/resume polling, mid-task input, and reload final state.

## Task 5: Roots model and shell bookmarks

Implement per-conversation roots.

Core model already has `roots` in `meta.json`; fill the behavior:

- `Root { id, name, uri/path, kind, scope }` as a portable intent record.
- For local filesystem roots, shell stores a security-scoped bookmark keyed by conversation id and root id.
- Core stores only the logical path/URI and display metadata.
- Gateway exposes roots to downstream servers that ask for them.
- If a bookmark is missing or expired, show the missing external-folder error state and let the user re-pick.
- Roots are copied on fork as intent, but bookmark access may need revalidation in the shell.

Tests: attach root, remove root, fork copies root refs, missing bookmark state, roots/list to downstream server, path outside root denied, and no bookmark bytes in vault files.

## Task 6: UI surfaces

Add the visible pieces:

- App panels inline in transcript, with server attribution and disabled/offline state.
- Consent cards for App-initiated actions.
- Task cards with status, progress, cancel, final result summary, and historical final state after reload.
- Roots picker in conversation settings/composer.
- Capability badges in settings: Apps, Tasks, Roots, Sampling declined.

Accessibility:

- App panel has fallback title/status/actions outside web content.
- Task cards are keyboard controllable and VoiceOver announces status changes politely.
- Roots picker works fully by keyboard.

Tests: UI state snapshots for app loaded/offline/action consent, task running/completed/failed, roots attached/missing, and sampling declined shown in capabilities.

## Task 7: Fixtures and tests

Extend fixtures:

- MCP App fixture server with a declared template, allowed origin, and bridge action.
- Task fixture server with progress, completion, cancellation, and mid-task input.
- Roots fixture server that requests/list roots and reads permitted paths through the gateway.
- RC capability fixture that exposes Apps/Tasks as extensions.

Enumerated tests:

1. `app_template_declared_origin_loads`.
2. `app_template_undeclared_origin_blocked`.
3. `app_resource_persists_and_reloads`.
4. `app_bridge_action_requires_consent`.
5. `app_bridge_denied_action_not_executed`.
6. `artifact_webview_still_has_no_bridge`.
7. `task_progress_updates_live_card`.
8. `task_completion_persists_task_ref`.
9. `task_cancel_routes_to_server`.
10. `task_survives_background_resume`.
11. `task_mid_input_uses_elicitation_path`.
12. `root_attach_persists_ref_not_bookmark`.
13. `root_missing_bookmark_surfaces_error_state`.
14. `roots_exposed_to_downstream_server`.
15. `sampling_declined_cleanly`.
16. `rc_extension_capability_gate`.

## V1 notes (reconciliation)

- **Knowledge-base roots are model-only in V1.** The core `Root` type supports `KnowledgeBase` alongside `Filesystem`, but the shell roots picker attaches folders only. KB URI roots can be stored in `meta.json` for forward compatibility; there is no KB picker or bookmark flow until a later milestone.
- **App distribution packaging is deferred to M9.** Milestone 7 wires App sandbox, bridge consent, and offline transcript rendering; signing, notarization, and update packaging stay in the ship milestone.

## Out of scope this milestone

Do not build product onboarding/search/share polish (M8). Do not add cloud sync or team roots. Do not add sampling. Do not build a general browser. Do not allow Apps to use undeclared network origins or bypass consent for host actions.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-5.md, docs/milestone-6.md, docs/milestone-7.md, and docs/tamtri-decisions.md sections 9, 10, and 15. Implement Milestone 7. Start by factoring the webview policy for artifact vs App rendering and adding capability gates, then stop and show me the App sandbox/bridge design before implementing Tasks and Roots. Artifacts remain no-network and no-bridge; Apps get only declared origins and consent-gated host calls.
