# Daemon wire protocol

tamtri surfaces (Electron desktop, web, mobile, CLI) talk to a single headless
**tamtri-daemon** over a versioned JSON-RPC/WebSocket protocol. The Rust core
(`core/src/protocol`) is the source of truth; TypeScript types are generated via
typeshare into `packages/protocol`.

## Transport

- **Direct (localhost):** `ws://127.0.0.1:<port>/ws?token=<bearer>`
- **Remote (relay):** `wss://<relay-endpoint>/...` with E2E encryption after QR/URL pairing (see [relay-threat-model.md](./relay-threat-model.md)). The daemon maintains an outbound registration connection when `TAMTRI_RELAY_ENDPOINT` is set (disable with `TAMTRI_RELAY_DISABLE=1`).

Endpoint discovery: `~/.tamtri/daemon.port` and `~/.tamtri/daemon.token` (0600).

## Handshake

1. Client opens WebSocket with bearer token query param.
2. Client sends JSON-RPC request `hello` with `{ client_id, client_type, protocol_version, app_version? }`.
3. Daemon responds with `ServerInfo`: `{ server_id, version, protocol_version, features }`.

`features` is additive. Clients gate optional capabilities on these flags.

## Envelope

All frames are JSON-RPC 2.0 text over WebSocket:

- **Request:** `{ jsonrpc, id, method, params? }`
- **Response:** `{ jsonrpc, id, result?, error? }`
- **Notification (daemon → client):** `{ jsonrpc, method, params? }` — no `id`

Streaming UI events arrive as notifications on method `event` with an
`EventNotification` payload (`conversation_id`, `kind`, `payload_json`).

## Method registry

See `core/src/protocol/mod.rs` (`method` module) and the generated/hand-written
mirror in `packages/protocol/src/methods.ts`.

Notable groups:

| Domain | Examples |
|--------|----------|
| Conversations | `conversation.create`, `conversation.load`, `conversation.send_message`, `conversation.fork`, `conversation.export_bundle`, `conversation.import` |
| Projects | `project.list`, `project.create`, `project.update`, `project.delete`, `project.root_attach`, `project.root_remove`, `project.conversation_create`, `conversation.move_project` |
| Run control | `run.cancel`, `permission.respond`, `elicitation.respond`, `task.cancel` |
| Harness roster | `agents.list`, `agents.models`, `harness.providers_list`, `harness.roster_set_enabled`, `harness.roster_add`, `harness.health_list`, `harness.health_checklist`, `harness.usage_list` |
| Workdir & artifacts | `workdir.write_file`, `workdir.copy_file`, `attachment.read_verified`, `artifact.verify_inline` |
| Gateway | `gateway.list_servers`, `gateway.set_credential`, `gateway.start_oauth`, `gateway.complete_oauth` |
| MCP Apps | `app.resolve_template`, `app.submit_bridge_request`, `app.respond_bridge_consent` |
| Roots | `roots.attach`, `roots.list`, `roots.sync_runtime` |
| Search | `search.conversations`, `search.scope_message` |
| Orchestration | `recipes.list`, `orchestration.run`, `orchestration.status`, `orchestration.cancel` |
| Sessions | `sessions.list_native`, `sessions.import` |
| Vault & diagnostics | `vault.issues`, `vault.path`, `diagnostics.write_bundle` |
| Relay | `relay.pairing_offer` |

## Projects

Project support is advertised by `ServerInfo.features.projects`. Project methods return `ProjectDto` records with `id`, `name`, timestamps, and shared `RootDto` values.

| Method | Params | Result and behavior |
|--------|--------|---------------------|
| `project.list` | none | `ProjectDto[]`, including the stable Unfiled record |
| `project.create` | `{ name }` | Creates and returns a `ProjectDto`; names are trimmed and cannot be empty |
| `project.update` | `{ id, name }` | Renames and returns the project; Unfiled is immutable |
| `project.delete` | `{ id }` | Returns `null`; clears member conversations to Unfiled before deleting the project |
| `project.root_attach` | `{ project_id, name, uri, kind, scope }` | Returns a `RootDto` with `origin: "project"` |
| `project.root_remove` | `{ project_id, root_id }` | Returns `null` after removing the shared root |
| `project.conversation_create` | `{ project_id, title, harness_id, model_id }` | Creates and returns a `ConversationDto` in that project |
| `conversation.move_project` | `{ conversation_id, project_id }` | Returns the updated `ConversationDto`; transcript and files are unchanged |

`ConversationDto` and `ConversationSummaryDto` expose optional `project_id` plus `kind`. The daemon projects missing stored membership to the stable Unfiled id in DTOs without rewriting metadata. Passing that id to create/move stores no `project_id`, preserving the legible legacy representation. Unknown project or conversation ids return typed JSON-RPC errors.

Shared roots are resolved at run time as project roots followed by conversation roots, deduplicated by kind and URI. Export removes `project_id` and snapshots effective roots; inherited roots use `origin: "project_snapshot"` so bundles never reference a source-vault project.

## Credentials

The **daemon owns durable secrets**. Gateway credentials and OAuth tokens persist
in `~/.tamtri/credentials.sealed` (ChaCha20-Poly1305). On macOS the master key
lives in the login keychain (`dev.tamtri.credentials`); on other platforms it
falls back to `credentials.key` (0600).
Surfaces send values via `gateway.set_credential`; the daemon injects them
downstream and never writes raw secrets to `events.jsonl` or logs.

OAuth callback completion runs in the daemon via `gateway.complete_oauth`.

## Versioning

- Wire version: `PROTOCOL_VERSION` (currently `"1.0"`).
- Schemas are append-only: new fields are optional with defaults; never narrow or remove fields without a major bump (which we avoid).

Regenerate TS types after Rust protocol changes:

```bash
pnpm run protocol:generate
```
