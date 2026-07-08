# Daemon wire protocol

tamtri surfaces (Electron desktop, web, mobile, CLI) talk to a single headless
**tamtri-daemon** over a versioned JSON-RPC/WebSocket protocol. The Rust core
(`core/src/protocol`) is the source of truth; TypeScript types are generated via
typeshare into `packages/protocol`.

## Transport

- **Direct (localhost):** `ws://127.0.0.1:<port>/ws?token=<bearer>`
- **Remote (relay):** `wss://<relay-endpoint>/...` with E2E encryption after QR/URL pairing (see [relay-threat-model.md](./relay-threat-model.md))

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
| Conversations | `conversation.create`, `conversation.load`, `conversation.send_message` |
| Gateway | `gateway.set_credential`, `gateway.start_oauth`, `gateway.complete_oauth` |
| Relay | `relay.pairing_offer` |
| Sessions | `sessions.list_native`, `sessions.import` |

## Credentials

The **daemon owns durable secrets**. Gateway credentials and OAuth tokens persist
in `~/.tamtri/credentials.sealed` (ChaCha20-Poly1305 under `credentials.key`).
Surfaces send values via `gateway.set_credential`; the daemon injects them
downstream and never writes raw secrets to `events.jsonl` or logs.

OAuth callback completion runs in the daemon via `gateway.complete_oauth`.

## Versioning

- Wire version: `PROTOCOL_VERSION` (currently `"1.0"`).
- Schemas are append-only: new fields are optional with defaults; never narrow or remove fields without a major bump (which we avoid).

Regenerate TS types after Rust protocol changes:

```bash
npm run protocol:generate
```
