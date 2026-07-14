# Milestones 3–9: outline to a shippable tamtri

The path from "core library with tests" to a fully functional app. Milestones 1–2 are done. This outline is the map; each milestone gets a full spec doc (tasks, types, enumerated tests) before its build session, same as M1/M2.

UI shell direction: Swift owns the outer Mac app. React/TypeScript may be introduced as a contained `WKWebView` renderer island for transcript cards, artifacts, tables, diffs, code blocks, and MCP Apps. The renderer receives view state and emits intents; Swift/core keep ownership of vault, gateway, credentials, permissions, and harness lifecycle.

One change from the old build order: **artifact rendering moves ahead of elicitation** (old M6 → new M5). The hero demo needs the sandboxed webview and nothing from elicitation, and artifacts ride ACP `FileChanged`, not the gateway. The webview infrastructure lands once and MCP Apps reuse it later. Hero demo two milestones sooner.

On MCP coverage: "the majority of the protocol" is the bar, and it is concrete. By M7 tamtri covers tools, resources, prompts, elicitation (form + URL), Apps, tasks, roots, progress, cancellation, logging, pagination, and both transports (stdio + streamable HTTP), with OAuth for remote servers. The one deliberate exception is **sampling**: the model lives in the agent, not the gateway, so a downstream sampling request has no model to answer. tamtri declines the capability cleanly at initialize (servers see an honest capability set, nothing breaks). Revisit only if a target server needs it. That is a majority in every sense that matters, and the gap is principled, not lazy.

---

## M3: AcpAdapter + first light

The core meets the shell. Spawn a real agent, stream a real conversation, persist it.

- `HarnessAdapter` trait + `AcpAdapter`: spawn agent subprocess, ACP handshake, `session/new` with `cwd` (workdir) and `mcpServers`, normalize `session/update` into `HarnessEvent`s.
- Stream reduction: deltas buffer and commit as `ContentBlock`s per the M1 rules (one line per completed message). `events.jsonl` starts recording.
- Permission requests render as consent cards per the CLAUDE.md consent contract (who, what, full diff or exact command, scope choices).
- UniFFI bridge: async Rust core to Swift, first and hardest FFI pass.
- Minimal Swift outer shell: sidebar (list from vault), transcript renderer for text/thinking/tool cards, composer with drag-and-drop into `workdir/`. The transcript can be SwiftUI at first or an initial WebKit renderer island if that is faster.
- Done when: a conversation with Claude Code (or any ACP agent) runs end to end in the app, survives quit and relaunch, and redraws from `messages.jsonl` alone.

## M4: The gateway + the full MCP client

tamtri takes its place in the middle. The largest core milestone.

- Promote the M2 client to the **multiplexed dispatch loop**: background reader, pending-request map, inbound-request channel. Public API unchanged (that was the point of `&self`).
- **Streamable HTTP transport** beside stdio. Remote servers are half the MCP ecosystem; a local-only gateway is not the thesis.
- MCP **server** surface toward the agent; proxy `tools/*` downstream with **progress, cancellation, and logging passthrough** so long tool calls feel alive in the UI.
- Proxy **resources and prompts** (list/read/get, pagination). Subscriptions are a stretch goal, not a blocker.
- Downstream **server registry** in vault-level config + minimal settings UI, with the gateway-tools vs agent-native-tools split and scope labels.
- **Credential injection v1**: static secrets (API keys, bearer tokens) from the macOS keychain into downstream env/headers. The agent never sees a value; the audit log records "injected for server X."
- Fork-into-harness: fork picker, `ContextSeed` handoff to a different agent id.
- Done when: an agent calls a tool through the gateway against a local stdio server and a remote HTTP server, progress streams to a card, and `events.jsonl` shows the receipts.

## M5: The rendering plane (the hero)

The demo that says it all, plus the security infrastructure everything rich reuses.

- Sandboxed `WKWebView` host: **zero network for artifacts**, pre-declared origins only for Apps (built now, used by Apps in M7). If a React renderer island exists, artifact/App frames live inside that shell with the same sandbox policies. Accessible fallback per the UI spec.
- `FileChanged` → snapshot bytes to `attachments/` (content-hashed) → `Artifact` block → inline render. Validator from M1 is the only gate.
- Non-HTML artifacts get sensible cards: markdown rendered, CSV as a table preview, images inline, everything else a typed file card with open-in-Finder.
- Done when: drag a CSV into the composer, ask for a report, and `report.html` renders inline in the conversation. Record it; that clip is the launch.

## M6: Elicitation + remote auth

The follow-up question, rendered natively, and the credential story completed.

- Gateway intercepts downstream elicitation. **Form mode**: JSON schema to native SwiftUI form, accept/decline/cancel round-trip, `ElicitationRequest`/`Response` blocks persisted. **URL mode**: consent sheet showing the exact target host, then browser handoff.
- **OAuth 2.1 for remote HTTP servers**: authorization flow via browser handoff (same trusted-domain consent machinery as URL elicitation, built once), tokens in keychain, silent refresh. This is why auth lives here and not M4.
- Elicitation cards nest under the originating tool call via `origin_tool_call_id`.
- Done when: a downstream server elicits mid-tool-call, the user answers in a native form, the agent receives the finished result and never knew; and a remote OAuth server connects without the user touching a terminal.

## M7: Apps, Tasks, Roots

The remaining primitives, on infrastructure that already exists.

- **MCP Apps**: pre-declared templates rendered in the M5 webview host or React renderer island, JSON-RPC UI-to-host bridge, explicit consent for UI-initiated tool calls (same consent path, same audit log).
- **Tasks**: long-running work as live task cards, status polling, mid-task input, non-blocking transcript. `TaskRef` blocks persist final state.
- **Roots**: per-conversation roots picker, security-scoped bookmarks on the shell side, roots exposed to servers through the gateway.
- Gate 2026-07-28 RC behaviors (stateless core, Apps/Tasks as extensions) behind capability checks so both spec generations work.
- Done when: a server-returned App renders and interacts inside a message; a task survives the app being backgrounded; a root attaches and a server reads through it.

## M8: Product completeness

Everything CLAUDE.md's UI spec promises, delivered.

- First-run **harness health screen** (detect, status, install links, IT checklist).
- **Search** (titles + Text/Thinking, scope stated in the empty state). **Share/fork UX**: `.tamtri` export, import with hash verification and failed-integrity handling, fork lineage visible.
- All six **error states** with real copy. **Accessibility pass** against the V1 requirements (keyboard-first transcript, VoiceOver on every card, Reduce Motion, AA contrast, Dynamic Type).
- `issues()` surfaced: duplicate-folder badge, reveal in Finder. Diagnostics bundle ("Report an issue").
- Hotkeys, menu bar item, command palette, cold-start performance budget.
- Done when: a technical-adjacent user goes from download to hero demo without touching a terminal, and a VoiceOver user can run the same flow.

## M9: Ship it

Release engineering is a milestone, not an afterthought.

- Signing, notarization, DMG; Sparkle (or equivalent) auto-updates; Homebrew cask.
- CI: `cargo test` + `clippy` + Swift build + UI smoke on every PR; release pipeline from tag.
- Contribution mechanics live before the announcement: CLA + DCO, CONTRIBUTING.md, code of conduct, labeled good-first-issues per area.
- README built from the product brief, hero demo clip embedded, docs site seeded from `/docs`.
- v0.1.0: launch with the report demo.

---

## MCP coverage after M7

| Feature | Status | Where |
|---|---|---|
| Tools | full, proxied | M2 client, M4 gateway |
| Resources / prompts | proxied (subscriptions stretch) | M4 |
| Progress / cancellation / logging | passthrough | M4 |
| stdio + streamable HTTP | both | M2, M4 |
| Auth (static + OAuth 2.1) | full, keychain-backed | M4, M6 |
| Elicitation (form + URL) | full, gateway-intercepted | M6 |
| Apps | full, sandboxed | M5 infra, M7 |
| Tasks | full | M7 |
| Roots | full | M7 |
| Pagination | full | M2 |
| Sampling | declined at initialize, by design | revisit on demand |
| Completions | stretch, with prompts | M4+ |

---

## Gaps to track

- **M3 is oversized.** It combines the shared RPC loop, ACP adapter, reducer, event log, UniFFI, and first Swift shell. Treat the core ACP substrate as the first checkpoint, then the generated-binding Swift app as the second checkpoint. If a WebKit renderer island starts here, keep it renderer-only.
- **Run manager lifecycle.** The outline needs a durable in-memory run registry: one active run per conversation, cancellation on app quit, permission response routing, and cleanup after crashes.
- **Vault-level config schema.** M4 references global config for harness roster, downstream MCP servers, credential references, and timeouts. Specify the file path, schema, migration story, and validation rules before implementing settings UI.
- **Real-agent compatibility matrix.** Add a manual verification matrix for Claude Code ACP, Gemini, Goose, and one failure case. Track initialize/session fields, permission behavior, tool update shapes, and cancellation behavior.
- **Security review checkpoints.** Add explicit review gates before M5 webview rendering and M6 OAuth: sandbox entitlements, allowed origins, CSP strategy, keychain access groups, and audit-log redaction.
- **Database/index sequencing.** M8 search may not need SQLite, but if it does, define the rebuildable index schema and corruption behavior before UI work depends on it.
- **Release asset and docs ownership.** M9 should name the README/docs-site source of truth and how product brief, architecture docs, and generated API docs stay in sync.
