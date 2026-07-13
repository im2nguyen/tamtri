# CLAUDE.md

Project instructions for Claude Code. Read this before making changes. Keep it updated as the codebase evolves.

Project name: **tamtri**. Core crate: `tamtri-core`. License: **AGPL** (with a contributor CLA, to keep a future MIT relicense option open).

## What this is

An open-source, cross-surface agent UI shell for pluggable harnesses, with first-class rendering of modern MCP features (Apps, Elicitation, Tasks, Roots; Sampling deferred) and of artifacts a harness produces. Think "open Claude Desktop / open Codex": the harness and model are swappable, and the client owns the conversation.

Who it's for: **technical-adjacent knowledge workers**, not consumers and not terminal-native engineers. The marketer, analyst, ops, or PM who has agent tools available (Codex, Claude Cowork, Claude Code) and wants to turn a dataset into a report, but is far more comfortable in a UI than a terminal.

Hero use case: **report from data, not code.** "Turn this CSV into a report" and have tamtri render the report inline, instead of a terminal printing "I created report.html." tamtri is general-first; coding is just one thing the harnesses happen to do well.

Two protocols, two planes (see "Harness transport" and "MCP gateway" below):
- **Harness adapters** connect the daemon to agents. Heterogeneous registry: direct Claude/Codex native adapters for flagship fidelity; **ACP** is the long-tail fallback.
- **MCP** is owned by tamtri as a gateway. tamtri proxies the agent's tool calls and renders the rich primitives itself.

## Golden rules (do not violate)

1. **The client is a dumb shell.** It renders and stores. It does not implement an agent loop, model inference, or prompting strategy. All of that lives in a harness. **Exception:** the daemon may run a **prompt-free orchestration engine** that sequences harness runs (fork, send user-authored recipe text, wait, branch on structured signals). Coordination is not intelligence; the engine never generates prompts or model decisions.
2. **The conversation is the portable unit, and the client owns it.** Harnesses read a conversation's context and start a run. They never own the conversation. Harness and model are fixed for a thread; to change either you fork (see "fork to change either" below). This is what makes the experimentation surface work without betting on native cross-harness resume.
3. **The harness adapter interface is the future plugin contract.** Design every adapter as if a third party will implement it. No adapter may leak its process/parsing quirks past its own boundary; the core only ever sees normalized `HarnessEvent`s.
4. **Layer boundaries are sacred.** Surfaces (Electron + Expo/React-Native-Web) -> wire protocol -> Daemon (Rust core) -> Harness adapters. Nothing in a surface talks to a harness process directly. Nothing in the core imports Electron, React Native, or renderer UI code.
5. **Security is not optional.** Model-generated HTML runs sandboxed. This covers both MCP Apps and harness-produced artifacts (e.g. a `report.html` written by the harness). Harness-written artifacts render with **no network access at all** (a report must be self-contained; no CDN fetches, even for the hero demo). MCP Apps may reach only their pre-declared origins. UI-initiated actions go through the same consent/audit path as direct tool calls. Never route secrets through elicitation form mode; use URL mode to a trusted domain.
6. **Storage is a legible vault, not an app database.** Conversations live as user-visible files in a folder the user owns, openable in Finder, syncable via their own iCloud/Dropbox/Git. Never make storage opaque. This legibility is a core trust promise, not an implementation detail.
7. **tamtri owns the capability plane.** It is the MCP gateway for rich primitives (Apps, elicitation, tasks). The agent connects to tamtri as its one MCP server; tamtri proxies downstream. Never depend on the harness to carry or render these primitives.

## Architecture

Monorepo. Rust **tamtri-daemon** owns the vault, MCP gateway, harness processes, durable credentials, and wire protocol. Every surface is a thin client over WebSocket (localhost) or E2E relay (remote).

```
Surfaces (thin clients)              ← Electron desktop, web, mobile (Expo RN Web)
  @tamtri/client over WS or relay
Daemon (tamtri-daemon, Rust)         ← single writer, credential owner
  TamtriCore: vault, gateway, dispatch
Harness Adapters (Rust)              ← behind HarnessAdapter trait
  ClaudeNative, CodexNative, AcpAdapter (fallback)
/packages                            ← TS: protocol, client, relay, app, desktop
```

See `docs/daemon-protocol.md`, `docs/relay-threat-model.md`, `docs/provider-adapters.md`, `docs/orchestration.md`.

Surfaces are dumb: they render and emit intents. The daemon owns storage, gateway secrets, permission audit, and harness lifecycle.

## Tech stack

- Core + daemon: Rust (`tamtri-core`, `tamtri-daemon`). Wire protocol in `core/src/protocol`; types shared via typeshare.
- Surfaces: TypeScript. Electron desktop shell (`packages/desktop`); shared UI via Expo + React Native Web (`packages/app`, later).
- Client SDK: `@tamtri/client` (WebSocket + relay E2EE transport).
- Local persistence: legible vault under `~/.tamtri/vault`. Daemon runtime: `~/.tamtri/` (token, port, sealed credentials, relay keypair).
- No cloud, no accounts, no telemetry in V1.

## Repo layout

```
/core            Rust: TamtriCore, protocol, vault, MCP gateway, harness adapters
/daemon          Rust: tamtri-daemon binary (axum WebSocket server)
/packages
  /protocol      typeshare-generated wire types + JSON-RPC helpers
  /client        DaemonClient SDK
  /relay         E2EE relay crypto (tweetnacl)
  /app           Expo RN Web UI (desktop/web/mobile — in progress)
  /desktop       Electron shell (spawns daemon, IPC bridge)
/docs            architecture + protocol + adapter docs
```

## Core abstractions

### Conversation (owned by client, portable)
`Conversation { id, title, timestamps, project_id?, kind, messages[], active_harness_id, mcp_servers[], roots[] }`
`Message { id, role, harness_id?, content: ContentBlock[], created_at }`
`ContentBlock = Text | Thinking | ToolCall | ToolResult | AppResource | Artifact | ElicitationRequest | ElicitationResponse | TaskRef`

Storage layout (the vault): legible project metadata plus one folder per conversation.
```
<vault>/projects/<slug>--<project-id>/
  meta.json        mutable, atomic: schema_version, id, name, timestamps, shared roots
<vault>/conversations/<date>-<slug>--<id-suffix>/
  meta.json        mutable, tiny: schema_version, id, title, timestamps, project_id, kind, active_harness_id, model_id, working_dir, mcp_servers, roots, forked_from
  messages.jsonl   append-only, exactly one Message object per line (the transcript)
  events.jsonl     append-only local audit log (receipts)
  attachments/      curated rendered artifacts (e.g. report.html), hashed, referenced by transcript Artifact blocks
  workdir/          harness working directory for VaultLocal (inputs + harness files); local, not in share bundle
```

Persistence tiers, all plain text, all user-owned:
- **Transcript (`messages.jsonl`) is the complete render source.** On its own it must redraw the conversation exactly as seen after the session closes: thinking, text, tool calls with results and diffs, artifacts, elicitation exchanges. The reduction from the live event stream into `ContentBlock`s is lossless with respect to anything rendered; only sub-message token deltas and pure protocol chatter are excluded. `Thinking` persists here (reading it after the fact is a feature). `ToolResult.output` carries the rendered content so cards redraw without the live stream. A permission the user approved persists in compact form (it was part of what they saw).
- **Audit log (`events.jsonl`) is the local receipts.** Permission resolutions in full, full tool args, which downstream MCP servers the gateway hit, command executions. It is the trust artifact for the gateway/credential story. Local, not portable: it never leaves in a share bundle unless the user explicitly opts in.
- **Raw ACP wire trace is ephemeral, opt-in debug only.** The literal `session/update` firehose. Not persisted by default.
- **Secrets never persist to either log.** The gateway records "injected credential for server X," never the value.
- **Portability:** the transcript is the portable/fork-seed unit; the audit log is local. Legible means unencrypted at rest (an encrypted vault is a future option, not MVP).

- `meta.json` holds conversation-level metadata and is rewritten freely (it is tiny).
- `messages.jsonl` is append-only. A new message is a one-line append. Never rewrite it in normal operation. This gives clean Git diffs when the vault syncs and natural streaming.
- While a harness streams a message, buffer it and commit exactly one line on completion. In-flight tokens do not hit the log.
- `Artifact` content blocks reference a file under `attachments/` by vault-relative path plus mime type, size, and a content hash. Small text artifacts may inline (UTF-8 text ≤ 32 KiB, `ARTIFACT_INLINE_MAX_BYTES`); binary and larger files go to `attachments/`.
- Vault robustness rules: `meta.json` writes are atomic (temp + rename); reads are lock-free and tolerate a torn final `messages.jsonl` line in memory, while the write path repairs it on disk (interior corruption is a hard error); folder names are cosmetic and the id in `meta.json` is the truth (renames break nothing); the folder `<id-suffix>` is the id in simple form (32 hex chars, no hyphens), not a truncated prefix, so rapid v7 forks get distinct folders; duplicate ids from Finder copies or sync conflicts resolve deterministically to the newest `updated_at` and are reported via `issues()`; concurrency is any-readers/one-writer-per-conversation via `flock` on that conversation's `messages.jsonl` (no vault-wide lock; browsing while another instance runs is supported).
- Fork = copy the folder, new id + `forked_from` in `meta.json`, new folder name. Mutating a fork never touches the original.
- Share/export = freeze the folder into a self-contained `.tamtri` bundle (zip of `meta.json` + `messages.jsonl` + `attachments/`). Snapshot artifact bytes at that moment and hash-verify at the bundle boundary. Legible and mutable while it is yours; immutable and integrity-checked the instant it leaves.
- `schema_version` in `meta.json` governs the format so shared bundles stay loadable as the model evolves. Leave a documented migration seam.
- Store `harness_id` on each assistant message so the transcript shows provenance.
- A stable immutable Unfiled project owns the UI projection of legacy conversations with no stored `project_id`. Shared project roots join conversation roots at run time. Export clears the vault-local project reference and materializes inherited roots as portable `project_snapshot` roots.

### HarnessAdapter (swappable driver, = future plugin contract)
```
id, display_name, capabilities
run(context: ConversationContext) -> AsyncStream<HarnessEvent>
available_models() -> [ModelInfo]
cancel()
```
`run` yields an async stream of `HarnessEvent`s. The full event vocabulary (the adapter-to-UI contract) is defined in "Event vocabulary" below.

`ConversationContext` is what the core hands an adapter to start a run:
```
ConversationContext {
  seed: ContextSeed,        // how to start the run from prior context
  working_dir: WorkingDir,  // where the harness may operate, consent-scoped
  roots: [Root],
  mcp_servers: [McpServerRef],
  model_id: String,
}
ContextSeed = FreshTranscript(messages[])   // v1: replay prior messages
            | HandoffBrief(summary)          // later: compact context document
```

**The adapter owns seed-to-harness translation.** The core hands over a normalized `ContextSeed`. `AcpAdapter` injects it via ACP (`session/prompt`); a future native adapter would inject it however that agent wants (a synthesized opening prompt, a written brief). The core never knows how a given harness likes to be seeded. This keeps harness quirks inside the adapter, per rule 3.

### Fork to change either (the switching model)
There is no mid-conversation harness or model switch. Harness and model are chosen when a conversation is created or forked, and are fixed for that thread. To use a different harness or model, fork the conversation. Fork = copy the folder, new id, set `forked_from`, set the new `active_harness_id` / `model_id`, and seed the new run with the parent's context. One concept, no exceptions.

- This dissolves capability-mismatch: a forked run starts fresh, so it never inherits a live MCP App or foreign tool state. The parent keeps its rendered artifacts as static history.
- If a harness natively supports in-session model swaps (OpenCode's `/model`), that is a later harness-level convenience, not a tamtri primitive. The tamtri primitive stays "fork to change either."

### WorkingDir (repo-optional, consent-scoped)
Harnesses need a working directory, not a git repo. Git is optional and only unlocks extras (diffs, checkpoints) later. Two modes:
```
WorkingDir = VaultLocal          // default: a workdir/ folder inside the conversation's own vault folder
           | External(path)      // user points at any directory, e.g. a project repo
```
- Default is `VaultLocal`: the harness's cwd is a `workdir/` folder inside the conversation's own vault folder, holding user inputs and all harness working files. When the harness produces a renderable artifact (via `FileChanged`), tamtri snapshots a copy into `attachments/` (content-hashed, referenced by an `Artifact` block). `workdir/` can be messy and stays local; `attachments/` is the curated set that travels in a share bundle. (There is a copy step; do not conflate the two folders.)
- `External` stores the logical path in `meta.json` for legibility and intent. The macOS shell separately holds a security-scoped bookmark (`NSURL` bookmark data) keyed by conversation id, so access survives folder moves and is sandbox-legal. The bookmark is platform-specific and binary; it never goes in the portable `meta.json`.
- Never symlink an external directory into the vault. Symlinks break or misbehave across iCloud/Dropbox/Git and can drag an external tree into the vault. Store a path, resolve at runtime, ask consent Claude-Code style on first filesystem action.
- On share/export, snapshot the specific rendered artifacts into `attachments/`. An external working tree never travels in a `.tamtri` bundle unless the user explicitly opts in.

### V1 adapter: AcpAdapter (the first adapter, covers the whole roster)
`HarnessAdapter` is the abstraction. The first and only V1 implementation is `AcpAdapter`, which speaks the Agent Client Protocol (JSON-RPC 2.0 over stdio) and thereby drives every ACP-capable agent at once. This is not "ACP instead of adapters"; it is "ACP is adapter number one, and it is high-leverage." A future native adapter for a non-ACP agent drops in beside it behind the same seam.

- `AcpAdapter` spawns each agent as a subprocess, speaks ACP, normalizes `session/update` into `HarnessEvent`s.
- `session/new` declares `cwd` (the working_dir) and `mcpServers`. tamtri passes itself as the one MCP server (see gateway) so it can proxy and render.
- Isolate all ACP framing and per-agent launch quirks inside `AcpAdapter`.

Default picker roster is **general-first**, not coding-first (hero is report-from-data). Favor general-capable ACP agents: Hermes (general personal agent), Goose (general, MCP-native), and Claude Code framed for general use (it is coding-branded but handles "turn this data into a report" fine). Which agents ship in the picker is config, not engineering.

Note on Claude Cowork: it is the persona's north star (Anthropic's knowledge-work agent), but as of now it is a closed first-party desktop app with no ACP or headless interface, so it is not a pluggable harness. Log it as "watch for an ACP/headless interface," do not build against it.

### Enriched adapters: ClaudeCodeAdapter = AcpAdapter + enrichment
Some harnesses are important enough to enrich past the ACP baseline. The rule for where code lives:

- **If it is in the ACP protocol, it lives in `AcpAdapter`.** Streaming text, thinking, tool calls, `tool_call_update` content (including edit/write diffs and file changes), permission requests, terminal output, plan updates (when a harness emits ACP `plan`), lifecycle. These are baseline and every ACP agent gets them. The artifact hero rides here: file writes surface as standard ACP tool-call content, so `AcpAdapter` emits `FileChanged` generically, not per-harness.
- **If it is a harness-specific convention outside the protocol, it lives in a composed native adapter.** Example: Claude Code never sends ACP `plan`; it expresses steps via `TodoWrite`-shaped tool calls. Recognizing that tool name and emitting `PlanUpdated` is Claude-specific, so it belongs in `ClaudeCodeAdapter`, not `AcpAdapter`. Same for reading Claude Code's config to populate the capabilities panel.

`ClaudeCodeAdapter` **composes** `AcpAdapter` (decorator, not fork): it consumes the base adapter's normalized `HarnessEvent` stream (with a peek at pass-through raw payloads where needed), and emits enriched `HarnessEvent`s. The core still only ever sees `HarnessEvent`s, so rule 3 holds. This keeps two rates of change apart: `AcpAdapter` changes with the ACP spec; `ClaudeCodeAdapter` changes with Claude Code, without destabilizing the path that serves every other agent.

Capability ladder in the picker: **ACP-baseline harnesses** (Gemini, OpenCode, …) get whatever vanilla ACP carries; **enriched harnesses** (Claude Code first) get the baseline plus a native enrichment layer. Write enrichment only for the harnesses that carry the product.

### Event vocabulary (the adapter-to-UI contract)
`HarnessEvent` is the **superset** of textures any harness might surface. Each adapter emits only the subset it can; the renderer handles each variant uniformly regardless of which adapter produced it. Same variant, multiple possible sources (e.g. `PlanUpdated` from ACP `plan` OR from `ClaudeCodeAdapter` recognizing `TodoWrite`).

```
HarnessEvent =
  // baseline: AcpAdapter emits these from standard ACP
  | TextDelta { text }                                   // agent_message_chunk
  | ThoughtDelta { text }                                // agent_thought_chunk
  | ToolCallStarted { id, name, kind: ToolKind, title, input }        // tool_call
  | ToolCallProgress { id, status: ToolStatus, content: [ToolContent] } // tool_call_update
  | FileChanged { tool_call_id, path, change, diff: Diff }   // derived from ACP tool-call content; feeds artifact/vault
  | PermissionRequested { request_id, action, detail: PermissionDetail, options }
  | TerminalOutput { tool_call_id, chunk }
  | PlanUpdated { steps: [PlanStep] }                    // ACP plan when emitted; else synthesized by an enriched adapter
  | ModeChanged { mode }
  | Error { message }
  | TurnEnded { reason: EndTurn | Cancelled | Failed | MaxTokens }

ToolKind        = Read | Edit | Write | Execute | Search | Fetch | Think | Other(str)
ToolStatus      = Pending | InProgress | Completed | Failed
ToolContent     = Text(str) | Diff(Diff) | Json(value) | ResourceRef(uri)
Diff            = { path, change: Created | Modified | Deleted, old_text?, new_text? }
PermissionDetail= FileEdit(Diff) | Command(str) | Other(value)
PlanStep        = { title, status: Pending | InProgress | Completed }
```

`GatewayEvent` is the sibling contract for the capability plane (produced by the MCP gateway from downstream servers, not by a harness):
```
GatewayEvent =
  | ElicitationRequested { origin_tool_call_id?, server, request_id, mode, message, schema?, url? }
  | AppReturned          { origin_tool_call_id?, server, uri, template_ref, state }
  | TaskStarted | TaskUpdated | TaskCompleted { task_id, status, result? }
  // SamplingRequested — deferred
```

Correlation and reduction:
- Gateway events carry `origin_tool_call_id` when they occur during an agent tool call, so the UI nests an elicitation form or App panel under the right tool card.
- The core merges both streams (harness plane + gateway plane) and reduces them into the persisted `ContentBlock` model: `TextDelta`→`Text`, `ThoughtDelta`→`Thinking`, `ToolCallStarted`/`Progress`→`ToolCall`/`ToolResult`, `FileChanged`→`Artifact` (snapshot bytes to `attachments/`), `ElicitationRequested`→`ElicitationRequest`, `AppReturned`→`AppResource`, `Task*`→`TaskRef`. Streaming deltas collapse into one committed block; the transcript stays a complete, replayable render source.
- `PlanUpdated` is in the vocabulary for completeness but has no `ContentBlock` target in MVP (plan UI deferred). The MVP renderer may drop it; add a `Plan` content block when plan UI ships. This is the superset principle in action: the vocabulary carries the texture, rendering and capability vary.

## MCP gateway (the capability plane)

tamtri is an MCP gateway. It registers itself as the single MCP server the agent connects to (via `session/new`'s `mcpServers`), and proxies to the real downstream servers.

- Role flip: toward the agent, tamtri is the MCP **server** (agent is its client). Toward downstream servers, tamtri is the MCP **client**. A proxy in the middle.
- Flow: agent calls a tool → tamtri-gateway → downstream server → result back to the agent. If the downstream server elicits or returns an App, tamtri intercepts it, renders natively to the user, and returns the finished result to the agent. The agent sees an ordinary tool call.
- Why: server-initiated primitives (elicitation, Apps) are unpredictable. Being in the path of every tool call is the only way to guarantee tamtri catches and renders them, independent of harness maturity.
- Bonus: single consent/audit choke point; tamtri holds credentials and injects them downstream so the agent never sees raw secrets.
- Costs: tamtri is on the hot path and implements three protocol surfaces (ACP client, MCP server to agent, MCP client to downstream). Some harnesses load their own MCP servers (e.g. Claude Code's project `.mcp.json`) that tamtri does not intercept; route the primitives tamtri must own through the gateway, let the agent keep its own coding-tool servers.
- Concurrency: agents issue parallel tool calls, so the gateway fans out concurrently via a multiplexed dispatch loop (background reader + pending-request map + inbound-request channel).

## MCP feature requirements (V1)

Implement against MCP 2025-11-25, gating 2026-07-28 RC features (stateless core, Tasks/Apps as extensions) behind capability checks so the app works against both. tamtri holds the MCP client role (via the gateway) for everything it renders.

| Feature | Client responsibility | UI |
|---|---|---|
| Tools | proxy harness tool calls to downstream servers, return results | inline tool cards |
| Elicitation | intercept downstream elicitation at the gateway; render follow-up; return accept/decline/cancel; show which server asks | form card (structured) or URL consent + handoff |
| MCP Apps | render server HTML in sandboxed webview; pre-declared templates; JSON-RPC UI→host; consent on UI-initiated calls | inline interactive panel, attributed |
| Artifacts | a harness writing a file surfaces as standard ACP tool-call content; `AcpAdapter` emits `FileChanged`, the core snapshots bytes to `attachments/` and renders inline in the same sandboxed webview path as MCP Apps. Baseline, not harness-specific. | inline rendered artifact, openable in Finder |
| Tasks | poll status, show progress, allow mid-task input, non-blocking | task card with live status |
| Roots | let user attach filesystem/kb roots per conversation, expose to servers | roots picker in conversation settings |
| Sampling | **deferred.** The model lives in the agent, not the gateway, so a downstream sampling request has no model to answer. Revisit only if a target server needs it. | — |

## UI spec (V1)

Expo + React Native Web owns the shared UI. Electron hosts the exported web surface on desktop, spawns the daemon, and provides narrow OS bridges. The same renderer runs on web and mobile. This section is the shell's contract.

**Primary window.** Left sidebar: project-only tree with nested conversation threads, search, project creation, and Settings. Main pane: a raised content card with a 46 px toolbar and a centered transcript/composer column. The transcript renders content blocks in order (text, thinking, tool cards, artifacts, Apps, elicitation, tasks). An optional right dock summarizes and previews artifacts, Apps, and tasks. Composer: multiline input, send, attach-root, and drag-and-drop file attachment (dropped files land in the conversation's `workdir/`; this is the first beat of the hero demo).

**Renderer boundary.** The daemon/core owns storage, permissions, credentials, and security decisions. The renderer receives view models and emits intents through `@tamtri/client`. It cannot read the vault directly, connect to harnesses, call MCP servers, access keychain values, or bypass consent. Every UI-initiated action returns to core and uses the same consent/audit path as a direct tool call.

**First-run: Agents & providers.** The target user is not terminal-native, but V1 depends on installed agent apps. On first launch (and from settings anytime): detected agents with install and auth status, install-doc links for missing ones, and a copyable IT/admin setup checklist. Detect and guide; never bundle or manage an agent install.

**Consent card contract.** The permission card is the trust product, so its contents are spec, not style. Every card shows: **who is asking** (which harness, or which downstream server via the gateway, by name), **what exactly** (the full diff for file edits, the exact command string for executions, tool name plus a readable argument summary otherwise), and **scope choices**: allow once, allow for this conversation, or allow this action for this folder / this server. No global forever-allow in V1. Deny is always as prominent as allow. Every resolution persists: compact form in the transcript, full detail in `events.jsonl`.

**Tool provenance in settings.** Users will ask why a tool shows up in the agent but not in tamtri, or the reverse. The capabilities panel separates **tamtri gateway tools** (proxied, consent-gated, credential-brokered) from **agent-native tools** (the harness's own servers, e.g. a project `.mcp.json`), each labeled with its scope (system / user / project). Never blur the two; the distinction is the security model made visible.

**Fork-into picker, not a switcher.** There is no mid-thread harness or model switch (golden rule 2). The signature action is fork: a one-click "fork into…" affordance on any conversation opens a picker of harness + model and starts the forked thread seeded with the parent's context. Each conversation shows its harness/model as a chip; provenance across branches lives in fork lineage.

**Search scope (V1).** Search covers conversation titles and transcript text (`Text` and `Thinking` blocks). It does not search tool outputs, attachment contents, or `workdir/`. Say so in the search empty state rather than letting users infer it from missing results.

**Error states are designed, not raw.** Empty vault, malformed conversation, busy conversation (`ConversationBusy`), missing external-folder bookmark, unsupported schema version, and unavailable harness each get a calm state that names the problem and offers the one obvious recovery action (reveal in Finder, re-pick folder, update app, open Agents & providers). See `docs/product-gaps.md` for copy status.

**Accessibility (V1 requirement, not polish).** Full keyboard navigation of the transcript: every content block is focusable and traversable, with keyboard paths to card actions (expand diff, respond to elicitation, approve/deny consent). VoiceOver labels and values on every card type (tool, artifact, App, elicitation, task, permission). Honor Reduce Motion. Contrast meets WCAG AA. Respect Dynamic Type. Any web-rendered transcript/card surface must expose equivalent accessibility semantics or have native fallback metadata/actions outside the web content. Sandboxed model-generated webview content gets an accessible fallback (artifact title, type, open-in-Finder) since model-generated HTML cannot be trusted to be accessible.

**Desktop affordances.** The Electron shell owns window chrome, daemon lifecycle, file pickers, Finder reveal, and standard shortcuts. The renderer stays portable and reaches those capabilities only through the narrow preload bridge.

**Sharing / forking.** Export freezes the conversation into a `.tamtri` bundle (zip of `meta.json` + `messages.jsonl` + `attachments/`, hash-verified). Import a bundle or folder as a new conversation (new id, `forked_from` cleared). On import, verify every attachment hash: a mismatch imports the conversation but marks the affected `Artifact` blocks failed-integrity, names the files, and never renders their content (tampered HTML must not reach the webview). Fork keeps `forked_from` and continues with any harness.

## Build order (historical)

Core gateway, harness adapters, artifact rendering, elicitation, Apps/Tasks/Roots, orchestration, and the Expo/Electron UI are implemented at varying levels of polish. Remaining product work: onboarding gate, packaged Mac release, relay remote access, accessibility pass, menu bar/command palette.

Historical build-session specs: [`docs/archive/milestones/`](docs/archive/milestones/). Current doc index: [`docs/README.md`](docs/README.md).

## Commands

Keep this section current as tooling lands:

```
# core + daemon
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -p tamtri-daemon

# TypeScript surfaces
pnpm install
pnpm run protocol:generate
pnpm run typecheck
pnpm run test
pnpm --filter @tamtri/desktop run build
pnpm --filter @tamtri/client run test
```

Always run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `pnpm run typecheck`, and `pnpm run test` before considering surface work done.

## Code style

- Prose in docs and comments: short sentences, active voice, consequences first, every word earning its place. No em dashes.
- Rust: idiomatic, `clippy`-clean, errors are typed (no `unwrap()` in non-test code paths).
- Swift: idiomatic SwiftUI, no force-unwraps in view logic, keep views small and composable.
- TypeScript/React renderer code: renderer-only. Keep it deterministic, typed, sanitized, and free of vault/gateway/credential ownership.
- Prefer clarity over cleverness. Small, reviewable commits.

## What NOT to do

- Do not put agent-loop, prompting, or inference logic in the client.
- Do not let a harness own or mutate conversation storage.
- Do not couple the core to SwiftUI or any macOS API.
- Do not put vault logic, gateway routing, credential access, permission decisions, or harness process control in the web renderer.
- Do not make storage opaque. No content-addressed blob soup, no hidden database as the source of truth. The vault stays legible.
- Do not rewrite `messages.jsonl` on ordinary appends. Edits/deletes of past messages are the rare exception, not the pattern.
- Do not render harness-produced HTML or MCP App HTML outside the sandboxed webview + consent path.
- Do not give a rendered artifact network access. Artifacts are self-contained; only MCP Apps get network, and only to pre-declared origins.
- Do not depend on the harness to carry or render rich primitives (Apps, elicitation, tasks). tamtri owns them via the gateway.
- Do not let the agent connect directly to the MCP servers whose primitives tamtri must render; route those through the gateway.
- Do not bypass the consent/audit path for any UI-initiated action.
- Do not add cloud, accounts, or telemetry in V1.

## Out of scope (V1) — informs boundaries, do not build yet

Harness plugin system (promoting `HarnessAdapter` into a public contract + SDK; in practice mostly "more adapters and agents speaking ACP"), cross-platform native shells (GTK/WinUI reusing the Rust core), server discovery / MCP marketplace, team collaboration / cloud sync, extended long-task context management, MCP sampling.
