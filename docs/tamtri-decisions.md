# tamtri: Architecture Decisions and Rationale

A reference for what tamtri is, what we decided, and why. Read this to get back up to speed in a new conversation. Companion to CLAUDE.md (which is the operational spec for building) and the milestone docs (which are the build plans). This doc is the "why."

Last updated: July 2026. (The original product-spec.md is retired: CLAUDE.md is the operational spec, this doc is the why, the milestone docs are the build plans. The spec's surviving UI content moved into CLAUDE.md's "UI spec (V1)" section.)

---

## 1. What tamtri is

An open-source, native macOS desktop app that is a model-agnostic UI shell for pluggable agent harnesses, with first-class support for modern MCP features (Apps, Elicitation, Tasks, Roots; Sampling deferred, see section 15) and for rendering artifacts a harness produces.

One-line framing: **an open Claude Desktop / open Codex.** You get the Claude Code / Codex kind of experience (an agent that asks follow-ups, uses tools, iterates) but the harness and model are yours to choose, the app is open source, and it renders rich output a terminal cannot.

Signature demo: ask the agent (through tamtri, wrapping a harness like Claude Code) to create a report. Instead of "I created report.html," tamtri renders the report inline. That is the thing a terminal structurally cannot do, and it is the hook everyone immediately understands.

Positioning one-liner: **Conductor is "many agents, one codebase." tamtri is "one conversation, any agent, any tool, rendered richly."**

Who it's for: **not consumers, and not terminal-native engineers. The target is technical-adjacent knowledge workers** — the marketer, analyst, ops, or PM inside a company who has access to agent tools (Codex, Claude Cowork, Claude Code) and wants to, say, turn a dataset into a report, but is far more comfortable in a UI than a terminal. They understand what an agent is; the terminal is the barrier, not the concept. tamtri gives them the agent experience in a real app, with model freedom and open source on top. This persona also softens the onboarding problem: in a company, harness installs and credentials are often provisioned by IT rather than set up by the end user.

---

## 2. The market bet

- Terminal harnesses proved the demand for open, model-agnostic Claude Code alternatives (OpenCode alone crossed ~180k GitHub stars and millions of monthly active developers). But terminals are text-only: they cannot render MCP Apps, dashboards, forms, or a generated HTML report, and they shut out technical-adjacent knowledge workers (marketers, analysts, ops, PMs) who have agent tools available but will not live in a terminal.
- Vendor desktop apps (Claude Desktop, Codex) have the rich experience but lock you to one model provider.
- No open-source, native desktop client combines model freedom, rich MCP-primitive rendering, and accessibility for people who do not live in a terminal.

That gap is tamtri.

MCP primitives are becoming table stakes. The 2025-11-25 spec formalized elicitation and tasks; MCP Apps became the first official extension in January 2026; the 2026-07-28 release candidate is the largest revision since launch (stateless core, Apps/Tasks as versioned extensions). The bet is not that today's open models match Claude Code's reasoning. The bet is that these primitives become expected of every capable model, and whoever builds the cleanest surface for them wins mindshare when that happens. This is the VS Code / language-server pattern: the winning surface does not have to be the smartest thing in the room, just the best host of an open standard.

---

## 3. Product identity and monetization

**Decision: build the loveable product first, monetize the team/enterprise layer later.**

- Phase 1: an open-source, local-first, individual tool people love and trust. No accounts, no telemetry, no cloud.
- Phase 2 (only after real traction): team/collaboration and enterprise/SSO/self-host as the paid layer.

Why: when you are the client and not the model, there is no money in the app itself (you cannot charge for inference; tokens bill to the user's own keys). The durable paths are a paid team/collaboration layer, enterprise/self-host, or adjacent income (sponsorship, consulting, courses) in the Vue.js / Evan You mold. You earn the right to charge by being indispensable for free first.

Sequencing constraint: build V1 so the paid layer is an addition, not a rewrite. The portable conversation format and local-first design are what make a later sync/cloud layer a bolt-on rather than a teardown. Do not add analytics or accounts in V1 "to learn about users." Cloud enters only when a user opts into a team feature.

Honest caveat: the traction-to-enterprise jump is a different company (SOC 2, SLAs, sales). That is a later hire/raise/partner decision, not a V1 problem.

---

## 4. Core architecture

**Decision: Rust core + native Swift shell, with React/TypeScript renderer islands inside `WKWebView` where they materially speed up rich AI UI. One repo, bound via UniFFI. The Ghostty model for the core/shell split, with a WebKit renderer island for the hardest UI surfaces.**

Three layers, boundaries are sacred:
- Shell (SwiftUI + WebKit, macOS 14+): app lifecycle, windows, menus, hotkeys, vault picker, keychain, security-scoped bookmarks, consent surfaces, settings, composer, and sandboxed webview hosts. Replaceable.
- Core (Rust, portable): conversation model, vault, MCP client, harness manager. Platform-agnostic, reused by future shells.
- Harness adapters (Rust): per-harness drivers that spawn CLIs and normalize their output.

The rich AI surface can be a contained web renderer:
- React/TypeScript may own transcript card rendering, markdown, code blocks, diffs, JSON inspectors, CSV/table previews, charts, artifact frames, and MCP App frames.
- Swift owns the outer app and all security-sensitive decisions. The renderer receives view models and emits user intents. It never owns the vault, gateway, credentials, permission policy, or harness lifecycle.
- This keeps the shell native where Mac trust and integration matter, while using the web ecosystem where AI UI would otherwise require rebuilding a large amount of infrastructure in SwiftUI.

Why:
- **The client is a dumb shell.** It renders and stores. No agent loop, no inference, no prompting strategy in the client. All of that lives in a harness. This is the load-bearing principle; violating it collapses the whole design.
- **Native outer shell, not an Electron app.** The wedge is a fast, native-feeling Mac app (instant launch, menu bar, real shortcuts, keychain, bookmarks, system permissions) that also renders the new MCP primitives. Electron can be fast and is excellent for AI UI velocity, but it brings a larger runtime, more memory overhead, and a wider security surface. The compromise is Swift outside, WebKit renderer islands inside: no Electron runtime in V1, but React/TypeScript where it gives real leverage.
- **Monorepo now.** UniFFI Swift bindings are generated from the Rust core; splitting repos means a cross-repo version dance while you are a team of one. Enforce the boundary with directories, not repo walls. Ghostty itself is a monorepo (core + Swift macOS app + GTK Linux app). Extract the core later when a second shell (Linux/Windows) wants an independent release cadence.
- Fallback: if UniFFI iteration speed becomes a V1 blocker, an all-Swift core is acceptable, but the three layer boundaries stay identical either way.

Consequence: do not randomly split UI ownership. Swift is the outer app. Web views are contained islands. A future Electron shell remains possible because the Rust core boundary is clean, but V1 does not ship an Electron process.

---

## 5. Harnesses

> **Mechanism refined by sections 14 and 15.** The *selection* of harnesses below still stands, but they don't each get a bespoke adapter. `HarnessAdapter` stays the abstraction; the first adapter is `AcpAdapter` (section 14), which lights up every ACP agent at once, and tamtri acts as an MCP gateway (section 15). Read this section for which agents and why; read 14–15 for how they connect.

**Decision: V1 hardcodes adapters for OpenCode plus one general/MCP-native harness (Goose or Hermes, to be pinned before adapter #2). The adapter interface is designed as the future plugin contract.**

Definitions:
- A **harness** is the scaffolding around a model: the loop that reads files, calls tools, runs commands, and feeds results back. The model thinks; the harness drives.
- Conductor is **not** a harness. It is an orchestrator (a parallel runner for Claude Code / Codex / Cursor in isolated git worktrees). It is an adjacent neighbor, not something tamtri plugs in.

Selection criterion for a bootstrap harness: it must have a headless / programmatic mode drivable as a subprocess with a parseable stream. Good candidates: OpenCode (anchor, MIT, model-agnostic), Goose (MCP-native, Apache-2.0), Hermes (general personal agent, MIT), Codex CLI (`codex exec --json`), Crush.

Why OpenCode first: it is the de facto open, model-agnostic terminal agent, headless, MIT. Why a general/MCP-native second: it proves the "any harness, not just coding" thesis and (if Goose) stresses the MCP-App rendering path on day one. Note: Hermes is the better *story*, Goose is the better *test*. Open item: pin adapter #2.

Prior art for the adapter pattern: pi-builder already wraps installed CLIs behind one interface. It validates the abstraction; it is a library, not a UI.

MVP simplification: do not build the plugin system in V1. Hardcode two adapters. The `HarnessAdapter` interface *is* the future plugin contract, so promoting it to a public subprocess-over-stdio protocol + SDK is a V2 extraction, not a redesign.

---

## 6. The switching model: fork to change either

**Decision: no mid-conversation harness or model switch. Harness and model are chosen at create-or-fork and fixed for the thread. To change either, fork.**

Why:
- Native cross-harness *resume* (hand a Goose session to Claude Code and have it continue seamlessly) was the riskiest assumption in the project. Each CLI has its own session model and its own prompt-cache economics; a replayed foreign transcript is lossy and does not hit Claude Code's Anthropic-tuned caching. Betting the signature feature on that was fragile.
- "Fork to change either" is the honest version and it always works. You construct a context seed and start a clean run. It is also a *better* story: fork any conversation into any harness or model, keep the original intact, compare the branches. That is exactly the experimentation-surface thesis.
- It dissolves the capability-mismatch problem: a forked run starts fresh, so it never inherits a live MCP App or foreign tool state. The parent conversation keeps its rendered artifacts as static history. Nothing to reconcile.
- Provenance lives in fork lineage (`forked_from`) instead of per-message mixed provenance.

Mechanism: fork = copy the conversation folder, new id, set `forked_from`, set new `active_harness_id` / `model_id`, seed the new run with the parent's context. This is the existing fork primitive plus a harness/model argument.

The seed:
- V1: `FreshTranscript` — replay prior messages as starting context. Simplest, lossless, token-heavy but fine.
- Later: `HandoffBrief` — a compact summary the new run starts from (the same pattern Claude Code's own harness uses on context resets).

Critical boundary: **the adapter owns seed-to-harness translation.** The core hands over a normalized `ContextSeed`; each adapter decides how to inject it into its CLI. The core never knows how a given harness likes to be seeded.

Note on model swaps: if a harness natively supports in-session model switching (OpenCode's `/model`), that is a later harness-level convenience surfaced as a harness feature, not a tamtri primitive. The tamtri primitive stays "fork to change either." One concept, no exceptions.

---

## 7. Working directory: repo-optional, consent-scoped

**Decision: a conversation has a working directory, not a required git repo. Default is a folder inside the conversation's own vault; the user can point it at any external directory. Access is consent-scoped, Claude-Code style. Never symlink.**

Why:
- Neither Claude Code nor OpenCode requires *git*. They require a working directory and run in whatever folder you launch them from. Git is optional and only unlocks extras (diffs, checkpoints, PRs). The "must be a git repo" constraint belongs to Conductor (its worktree-isolation design), not to harnesses. tamtri is directory-based, so it is repo-optional and genuinely general-purpose, not "coding tool with extras."
- The Claude Code permission model is the right pattern: ask before touching the filesystem, scope access to the working directory.

Two modes:
- `VaultLocal` (default): a `workdir/` folder inside the conversation's own vault folder, holding user inputs and all harness working files. When the harness produces a renderable artifact, tamtri snapshots a content-hashed copy into `attachments/` (a real copy step; see sections 8 and 18). Syncs and shares cleanly.
- `External(path)`: the user points at any directory (e.g. a project repo). The logical path is stored in `meta.json` for legibility and intent.

Why not symlink an external directory into the vault: symlinks break or misbehave across iCloud (unreliable), Dropbox (follows the link and may drag the entire external tree into the vault), and Git (stores the literal target path, non-portable). A symlink also cannot travel in a share bundle. Store a path, resolve at runtime.

macOS specifics: raw absolute paths are brittle across moves, and a sandboxed Mac app cannot reopen an arbitrary path across launches. Use a **security-scoped bookmark** (`NSURL` bookmark data) held in the Swift shell, keyed by conversation id. It survives moves and is sandbox-legal. It is platform-specific and binary, so it lives in the shell, never in the portable `meta.json`. The core stores the logical path; the shell holds the durable access token. Keeps the core platform-agnostic.

On share: snapshot the specific rendered artifacts into `attachments/`. An external working tree never travels in a bundle unless the user explicitly opts in.

---

## 8. Storage: a legible vault, JSONL messages

**Decision: storage is a user-visible vault, one folder per conversation, with a small mutable `meta.json`, an append-only `messages.jsonl`, and an `attachments/` folder. Not an opaque database, not a content-addressed blob store.**

Layout:
```
<vault>/conversations/<date>-<slug>--<shortid>/
  meta.json        mutable, tiny: schema_version, id, title, timestamps, active_harness_id, model_id, working_dir, mcp_servers, roots, forked_from
  messages.jsonl   append-only, exactly one Message object per line
  attachments/     artifact bytes (e.g. report.html), referenced by vault-relative path
```

Why the vault (Obsidian model) over an opaque blob store:
- tamtri's whole wedge is trust: open source, local-first, no lock-in, your data is yours. A folder of sha256-named opaque files is technically local but not *legible*; it feels like an app database. Obsidian's loyalty comes from the opposite feeling: these are my files, I can open them in Finder, back them up with my own Git/iCloud, and walk away anytime. That feeling is worth more than what content-addressing buys.
- Artifacts already are files. When a harness writes `report.html`, it is a file. Round-tripping it into a hash-named blob just to render it is motion without benefit.

Why JSONL (not one big JSON array):
- Append-only: a new message is a one-line append. Cheap, and a torn final line is detectable and discardable.
- Clean Git diffs: adding a message is a one-line diff, not a reflow of a pretty-printed array. This matters because the vault syncs through the user's own Git/iCloud/Dropbox.
- Streaming-friendly. While a harness streams a message, buffer it and commit exactly one line on completion; in-flight tokens do not hit the log.

Why split `meta.json` from `messages.jsonl`:
- Conversation-level metadata (title, active harness, roots) is mutable. Putting it on a header line would rewrite line 1 on every change and break append-only purity. So mutable metadata lives in a tiny separate file that is cheap to rewrite; the message log stays purely append-only.

Index: any SQLite index is a rebuildable cache for fast listing/search, never a source of truth. The vault is the truth; the index can always be regenerated by scanning `meta.json` files.

Dedup and immutability (what a blob store would have given us):
- Dedup is a premature optimization for a personal tool. If it ever matters, content-address invisibly at the *cloud sync layer* later, without touching the legible local model.
- Immutability, one crisp rule: **anything the transcript renders is a content-hashed snapshot under `attachments/`, frozen at the moment it was rendered.** `workdir/` stays messy, mutable, and local. An old conversation therefore always redraws exactly what was seen, even if the harness later overwrites `report.html` in `workdir/`. **Snapshot-on-share** still applies at the bundle boundary: export freezes the folder into a self-contained `.tamtri` bundle and hash-verifies it, so a conversation is legible and yours while local, immutable and integrity-checked the instant it leaves.

Portability: share/export produces a `.tamtri` bundle (zip of `meta.json` + `messages.jsonl` + `attachments/`). Fork and import operate on the folder/bundle. This maps cleanly onto a future cloud layer: sync the manifest, push attachments to object storage, content-address there if desired.

Edge cases, decided (specified in milestone 1): folder names are cosmetic and the `meta.json` id is the truth, so renames break nothing; duplicate ids (Finder copies, sync "conflicted copy" folders) resolve deterministically to the newest `updated_at`, never auto-deleted, and surface via a `VaultIssue` report for later UI; `meta.json` writes are atomic (temp + rename); readers tolerate a torn final `messages.jsonl` line in memory and the write path repairs it on disk, while interior corruption is a hard error; concurrency is any-readers/one-writer-per-conversation (exclusive `flock` on that conversation's `messages.jsonl`, no vault-wide lock, so browsing a vault while another instance runs is supported); sync-conflict merges are the user's sync tool's domain, tamtri guarantees single-machine integrity. Still open: attachment bloat in a synced vault (set a size policy; consider keeping large binaries out of sync by default). Small text artifacts inline in the transcript only up to 32 KiB.

---

## 9. Rendering and security

**Decision: model-generated HTML runs sandboxed, and MCP Apps and harness-produced artifacts share the same sandboxed rendering + consent path.**

- MCP App HTML and a harness-written `report.html` are both model-generated HTML that could contain anything. Both render in a sandboxed `WKWebView`. Neither gets to touch the host except through the audited, consent-gated JSON-RPC path.
- The transcript/card renderer may also be React/TypeScript in a `WKWebView`, but it is trusted application code, not model-generated content. It still has a narrow bridge: view state in, user intents out. It does not get vault, gateway, credential, or permission authority.
- UI-initiated actions go through the same consent/audit path as a direct tool call.
- Elicitation: never request secrets via form mode; use URL mode to a trusted, displayed domain with consent before navigation.
- Consequence: the sandboxed webview is needed earlier than a naive "MCP Apps are milestone 6" reading suggests, because artifact rendering (the signature demo) also needs it.

---

## 10. MCP protocol posture

- Implement against MCP 2025-11-25. Gate 2026-07-28 RC features (stateless core, Tasks/Apps as versioned extensions) behind capability checks so the app works against both.
- Design the client's message loop so server-initiated requests (elicitation, and concurrent gateway traffic) can be handled while a call is in flight. Milestone 2's tools-only baseline uses simple sequential request/response behind an `&self` surface, parses server-initiated requests by field presence (answering `ping`, method-not-found for the rest), and isolates correlation in one place. **The multiplexed dispatch loop (background reader + pending-request map) is a milestone 4 requirement, not later**: agents issue parallel tool calls and the gateway must fan out concurrently, so the loop arrives with the gateway, before elicitation ever does.
- Keep tool-result content as raw JSON values for robustness against an evolving spec; it maps straight onto the conversation model's `ToolResult.output`.

---

## 11. Build order (milestones)

Re-sequenced twice: first for the ACP + gateway model (sections 14–15), then to pull artifact rendering ahead of elicitation. The hero demo needs the sandboxed webview and nothing from elicitation, artifacts ride ACP `FileChanged` rather than the gateway, and the webview infrastructure lands once for Apps to reuse. Hero demo two milestones sooner. OAuth for remote servers pairs with URL elicitation (shared trusted-domain handoff machinery). Full outline in `/docs/milestone-3-9-outline.md`.

1. Core skeleton: conversation model + vault + fork/import round-trip tests. No UI. **Done.**
2. MCP baseline (downstream half of the gateway): tamtri connects to a local MCP server as an MCP client; tools end to end. **Done.**
3. `AcpAdapter` + first app light: first `HarnessAdapter`, consent cards, `events.jsonl` starts, first core-meets-shell via UniFFI. The transcript may begin as SwiftUI or as a WebKit renderer island; Swift still owns the app.
4. Gateway + full MCP client: multiplexed dispatch loop, streamable HTTP transport, MCP server surface to the agent, proxy tools/resources/prompts with progress/cancellation/logging passthrough, server registry, static credential injection, fork-into-harness.
5. Rendering plane: sandboxed webview host + artifact hero (`report.html` inline).
6. Elicitation (form + URL) + OAuth 2.1 for remote servers.
7. Apps + Tasks + Roots; RC features behind capability checks. (Sampling deferred, see section 15.)
8. Product completeness: onboarding, search, share/fork UX, error states, accessibility, diagnostics.
9. Ship: signing, notarization, updates, CI, contribution mechanics, v0.1.0.

---

## 12. Open questions (not yet decided)

- **Default harness roster:** general-first (hero is report-from-data), built from general-capable ACP agents: Hermes, Goose, and Claude Code framed for general use. Cowork is the persona north star but not pluggable today (closed, no ACP/headless interface); logged as "watch for an interface." Which ship in the picker is config, not engineering.
- **Capabilities introspection (next probe):** `agentCapabilities` is a thin window (the Claude Code spike found no `fs` or `plan` flags). Does `initialize` / `session/new` surface a harness's available skills and configured MCP servers, or only primitive flags? Determines whether the capabilities panel (Harness / Skills / MCP servers, tagged by system/user/project scope) can be pure ACP introspection or needs per-harness config readers. See section 16.
- **App content fidelity:** does ACP carry an MCP App's interactive resource back through the agent, or only static references? Determines whether agent-invoked Apps render, or whether App-type servers must be gateway-owned. (Gateway-owned rendering is the fallback regardless.)
- **New-file create payload (quick check):** confirm a brand-new file (e.g. `report.html`, not an edit) arrives as a Write tool call carrying full content, since the spike leaned on edit hunks. This is the exact path the hero demo walks.
- **Pure-gateway vs hybrid:** default policy for agent-native MCP servers (e.g. Claude Code's project `.mcp.json`) that tamtri does not intercept. Lean: gateway owns the rich primitives; let the agent keep its own coding-tool servers.
- **License: AGPL now (decided).** Start AGPL, keep the option to relax to MIT later. Rationale: you can always relicense your own code more permissively, but you cannot claw back MIT grants, so AGPL-then-maybe-MIT is the reversible direction and MIT-then-AGPL is not. AGPL also deters proprietary forks of the surface while the project is young. **Consequence to honor:** to preserve the future MIT option, every external contribution needs a CLA or copyright assignment; otherwise contributors' AGPL code cannot be relicensed. Set up a CLA/DCO before accepting outside PRs. Note AGPL may complicate some corporate adoption and the future enterprise/self-host story, which is acceptable now given the protect-first intent.
- **Onboarding for technical-adjacent users (decided):** a first-run **harness health screen**. tamtri detects installed ACP agents (well-known binaries and config paths), shows per-agent install and auth status, links each agent's install docs, and offers a copyable IT/admin setup checklist for the common case where a company provisions the tools. tamtri detects and guides; it never bundles, installs, or manages an agent, which keeps it out of the inference-vendor lane. Spec'd in CLAUDE.md's UI spec; prototyped alongside milestone 3, polished in milestone 8.
- **Learning without telemetry (decided):** no passive metrics, ever, in V1. The project learns through an opt-in diagnostics bundle: a "Report an issue" action assembles app version, macOS version, harness roster and versions, and recent non-sensitive log excerpts into a local file the user reviews before attaching to a GitHub issue. Nothing leaves the machine on its own.
- **Contribution mechanics before launch (decided):** the CLA (required to preserve the MIT option) plus DCO, a CONTRIBUTING.md, a code of conduct, and labeled issues (good-first-issue per milestone) all exist before outside PRs are invited. The brief's "read your way to a first PR" promise depends on this.
- **Launch hero flow:** the single "wow" demo to build toward. Leading candidate is the report.html render.

---

## 13. Golden rules (the short list that must not be violated)

1. The client is a dumb shell. No agent loop, inference, or prompting in the client.
2. The conversation is the portable unit and the client owns it. Harness/model fixed per thread; fork to change either.
3. The harness adapter interface is the future plugin contract. Adapter quirks never leak past the adapter boundary. ACP-first: protocol textures live in `AcpAdapter`; a flagship harness gets a native adapter that *composes* `AcpAdapter` and enriches its event stream (e.g. `ClaudeCodeAdapter`), normalized to `HarnessEvent`s so the core stays agnostic.
4. Layer boundaries are sacred. Core never imports SwiftUI, WebKit, renderer code, or any platform API.
5. Model-generated HTML runs sandboxed; UI-initiated actions are consent-gated.
6. Storage is a legible vault, never an opaque database.
7. tamtri owns the capability plane. It is the MCP gateway for rich primitives (Apps, elicitation, tasks); never depend on the harness to carry them.

---

## 14. Harness transport: ACP is the first adapter

**Decision: `HarnessAdapter` stays our abstraction. The first adapter we write is `AcpAdapter` — one adapter that speaks the Agent Client Protocol and thereby lights up every ACP-capable agent at once (Claude Code, Gemini, OpenCode, Codex, Hermes, Goose, and ~25 more). Later adapters can wrap non-ACP agents natively, or become the plugin surface others target.**

Framing matters here: this is not "ACP instead of adapters." It is "ACP is adapter number one, and it happens to be enormously high-leverage." We keep the seam; we just get most of the ecosystem from a single implementation of it.

Why ACP as that first adapter:
- All target harnesses already speak ACP: Claude Code (via Zed's SDK adapter), OpenCode (`opencode acp`), Codex (via adapter), Hermes, Goose, Gemini, plus ~25 more as of early 2026. One adapter covers the whole roster.
- It kills transport heterogeneity. Without ACP you would write one parser for Claude Code's stdio NDJSON, another for OpenCode's HTTP+SSE server, and so on — a separate adapter per harness. With ACP, that becomes a single `AcpAdapter`. JSON-RPC 2.0 over stdio for everyone. One message loop.
- It is "LSP for agents" — it solves the M×N integration problem. Every new ACP agent becomes a tamtri harness for free through the same adapter. This is the ecosystem-leverage version of the plugin thesis; the V2 "harness plugin system" is largely "more adapters, and/or agents speaking ACP."
- Its model fits tamtri. The client drives; permission and filesystem access route through the client (`session/request_permission`, client-side fs read/write). That matches the consent + vault + working_dir design exactly.
- Stewardship risk is acceptable (see the stewardship note above).

Mechanism:
- `AcpAdapter` is the ACP client. It spawns each agent as a subprocess and speaks JSON-RPC over stdio, normalizing to `HarnessEvent`s like any adapter.
- `session/new` declares `cwd` (the working_dir) and `mcpServers`. The client chooses which MCP servers the agent sees — the hook the gateway relies on (section 15).
- Because `AcpAdapter` sits behind the `HarnessAdapter` seam, ACP's coding-centric worldview never leaks into the core, and a future native adapter (for a non-ACP or general agent) drops in beside it without disturbing anything.

Consequences:
- Milestones collapse. Writing `AcpAdapter` lights up every ACP harness at once; fork-into-harness becomes "launch a different agent id with the seed."
- Which agents ship in the default picker is a config/registry choice, not an engineering effort per harness.

---

## 15. Capability plane: tamtri as MCP gateway

**Decision: tamtri is an MCP gateway. It registers itself as the single MCP server the agent connects to, and proxies to the real downstream servers. Tool results flow back to the agent; interactive primitives (Apps, elicitation, tasks) surface to the user through tamtri.**

Why a gateway rather than letting the agent own MCP directly:
- Server-initiated primitives are unpredictable. You cannot know in advance which of the agent's tool calls will trigger an elicitation or return an App. The only way to guarantee you catch every one is to be in the path of all of them. Dual-connection (agent owns most servers, tamtri owns a few known UI servers) only catches what you predicted. The gateway catches everything, because everything flows through it.
- It decouples tamtri's signature primitives from harness maturity. Even a harness that does not implement elicitation or Apps as an MCP client still gets them, because tamtri is the MCP client. This fully dissolves the three-link dependency chain (model calls tool -> harness supports elicitation -> harness bridges it over ACP).
- It neutralizes the "ACP stays coding-centric" risk. Rich rendering rides tamtri's own MCP plane, not ACP.

Mechanism (the proxy sandwich):
- At `session/new`, tamtri passes one `mcpServers` entry: itself (a stdio or HTTP MCP endpoint tamtri hosts). ACP guarantees the client chooses the servers, and all agents must support stdio, so this is portable across every harness.
- Role flip: toward the agent, tamtri is the MCP **server** (the agent is its client). Toward downstream servers, tamtri is the MCP **client**. A proxy in the middle.
- Flow: agent calls a tool -> tamtri-gateway -> downstream server -> result back to the agent. If the downstream server elicits or returns an App, tamtri intercepts it, renders natively to the user, collects any answer, and returns the finished result to the agent. The agent sees an ordinary tool call.
- Precedent: `mcp-proxy-for-aws` and `acp-mcp-server` show MCP proxying is a solved, boring pattern.

Bonuses:
- A single consent and audit choke point for every tool call, harness-independent.
- Credential brokering. Because the client passes credentials to servers, tamtri holds them and injects them downstream. The agent never sees raw secrets. This is the MCP-gateway / credential-broker posture, and it is a real security property, not incidental.

Costs (accepted):
- **Sampling deferred (see below).** The model lives in the agent, not tamtri, so a downstream sampling request has no model to answer at the gateway.
- **Agent-native servers create a hybrid.** Some harnesses load their own MCP servers (e.g. Claude Code's project `.mcp.json`) that tamtri does not intercept. Stance: route the primitives tamtri must own through the gateway; let the agent keep its own coding-tool servers. Pure-gateway is cleaner but not always fully controllable.
- **tamtri is on the hot path.** It now implements three protocol surfaces at once: ACP client, MCP server to the agent, and MCP client to downstream. A gateway bug breaks tool use, so the robustness bar rises.

Sampling: deferred.
- What it is: the inverted primitive. Normally the agent calls a server's tools; sampling is the server calling back to ask the client's model to generate something mid-execution, so a server can be "smart" without shipping its own model or API key.
- Why deferred: it is the least-adopted primitive, most servers never require it, and it is architecturally awkward in the gateway topology (no model at the gateway). Revisit only if a target server needs it. Options then: a small tamtri-side model config just for sampling, declining the capability during MCP init, or routing the request to the agent's model if ACP grows a clean way to express that.

---

## 16. ACP fidelity findings — Claude Code

**Result: yellow, close to green. "Claude Cowork at home" through ACP is validated.** The spike's real question was not "does ACP expose everything Claude Code does" but "can tamtri deliver the Cowork-style knowledge-work experience by driving Claude Code over ACP." It can. The report-from-data hero works: streaming, structured tool calls with diffs, permission-before-write, terminal output, thinking, and a clean lifecycle all come through. The gaps are known and acceptable.

### Scorecard

| Texture | Came through? | Notes |
|---|---|---|
| Streaming text | Yes | Real token streaming (a reply arrived as several chunks). |
| File write surfaces | Yes, but not via ACP fs delegation | See key finding below. |
| Permission request | Yes | Fires before every write and terminal command, full diff/command in the payload, renders as consent with no extra lookup. |
| Plan / todo | No | Never appeared, even on an explicit "do X, then Y, then Z" prompt. No `plan` capability advertised, no `TodoWrite` calls. Flattens completely. |
| Tool call + result | Yes | Rich structured events for ToolSearch/Read/Write/Edit/Bash with status transitions. Real card material. |
| Clarifying follow-up | Not observed | The vague "improve the report" prompt did not trigger a question; Claude Code used judgment and proceeded. The streaming mechanism would carry one fine if it happened. |
| Thinking | Yes | Streams separately from message text, easy to show or hide. |
| Diff / edit detail | Yes, structured | Edits arrive as real `old_string` / `new_string` hunks, not whole-file dumps. |
| Terminal output | Yes | Verified with a real `tidy` validation command; actual stdout came back. |
| Error / cancel / done | Yes | Distinct signals: `status:"failed"` with message, `stopReason:"cancelled"`, `stopReason:"end_turn"`. |

### Key finding: file writes do not use ACP filesystem delegation

`fs/read_text_file` and `fs/write_text_file` are optional ACP client methods (the agent asks the client to touch the filesystem so the client stays in control). Claude Code's ACP adapter does not advertise or use them. `agentCapabilities` does not even list `fs`. Claude Code writes directly to `cwd` with its own internal Write/Edit tools.

That is fine, arguably better for tamtri. The write is fully visible anyway: it arrives as a `tool_call_update` (`kind:"edit"`) carrying a full diff, preceded by a `session/request_permission` carrying the same diff before the write lands. So tamtri gets the write, the full diff, and a pre-write consent hook, all as structured events.

(Correction to earlier loose phrasing: this is not "MCP," and there is no alternate fs channel in play. It is simply that Claude Code writes directly and reports via tool events rather than delegating to the client over ACP.)

### Decisions that flow from this

- **Artifact / vault hero rides `tool_call_update` diff content, not `fs/write_text_file`.** Snapshot artifact bytes to the vault from the tool-event diffs. Do not build the artifact path around ACP fs delegation; that channel is unused by this adapter.
- **Consent is unified.** Permission-before-write arrives cleanly with full payload, so the harness consent path and the gateway consent path share one native UI. The pre-write permission is a nicer consent moment than watching a filesystem.
- **No plan/todo UI for MVP.** ACP `plan` is dead for Claude Code. Thinking chunks already survive and give "it is reasoning" texture for free. Structured step-visibility, if ever wanted, comes from recognizing `TodoWrite`-shaped tool calls, which is harness-aware interpretation and stays inside the adapter (normalized to `HarnessEvent`s, per golden rule 3). Later enhancement, not a launch feature.
- **Clarifying follow-ups need no design.** "Not observed" is model behavior, not an ACP gap. The streaming mechanism carries a question fine if the model asks one.

### The strategic reframe: ACP-first, compose a native adapter where it matters

Claude Code is a flagship harness, big enough to justify a native adapter that enriches the ACP baseline. `AcpAdapter` stays pure vanilla ACP and unlocks the whole roster. `ClaudeCodeAdapter` **composes** `AcpAdapter` (decorator over its event stream, not a fork) and adds only the non-protocol Claude conventions.

The boundary is the ACP protocol line:
- **In ACP → `AcpAdapter` (baseline).** Streaming, thinking, tool calls, `tool_call_update` content including edit/write diffs and file changes, permission, terminal, plan-when-emitted, lifecycle. The artifact hero rides here: file writes are standard ACP tool-call content, so `AcpAdapter` emits `FileChanged` generically for every ACP agent, not just Claude Code. (Correcting earlier framing: "bytes from `tool_call_update` diffs" is ACP-standard, not Claude-specific.)
- **A harness convention outside ACP → the composed native adapter.** Claude Code expresses steps via `TodoWrite`-shaped tool calls, not ACP `plan`; recognizing that and emitting `PlanUpdated` is Claude-specific and lives in `ClaudeCodeAdapter`. Same for reading Claude Code's config for the capabilities panel.

This keeps two rates of change apart (`AcpAdapter` tracks the ACP spec; `ClaudeCodeAdapter` tracks Claude Code) and gives a clean capability ladder: baseline harnesses get vanilla ACP, enriched harnesses get baseline plus enrichment. Write enrichment only for the harnesses that carry the product. Full introspection through ACP is not required. The spike's purpose was "Cowork at home," and that is validated.

The shared event vocabulary (`HarnessEvent`, defined in CLAUDE.md) is the contract that makes this work: it is the superset of textures any harness might surface, so an enriched adapter emits variants the baseline cannot, while the renderer stays uniform.

### Consequence for the capabilities panel

`agentCapabilities` is a thinner window than hoped (no `fs`, no `plan` for Claude Code). So the planned capabilities view (Harness / Skills / MCP servers, each tagged by system / user / project scope) likely needs per-harness config readers rather than pure ACP introspection. Whether `initialize` / `session/new` surfaces a harness's skills and configured servers at all is the next probe (logged in section 12). tamtri reflects what the selected harness exposes; skills stay the harness's domain (install already asks which harness), and the panel's value is legibility, especially showing each capability's scope.

---

## 17. Persistence tiers and what the user sees

**Decision: everything durable is plain text in the user's vault, split into tiers by purpose. The transcript is a complete render source and is portable; the audit log is local receipts. Secrets never persist.**

The tempting answer, "store the whole ACP stream as plain text like `messages.jsonl`," is wrong: the ACP stream is a firehose (token deltas, status transitions, protocol chatter), and dumping it into the transcript destroys the "open it in Finder and read it" promise. So the split is by purpose, not by format. All tiers are plain text and user-owned.

### Tier 1: transcript (`messages.jsonl`) — the readable conversation, and the complete render source
- On its own it must redraw the conversation exactly as it looked live, after the session closes or the user switches away. Thinking, text, tool calls with results and diffs, artifacts, elicitation exchanges.
- The reduction from the live event stream into `ContentBlock`s is lossless with respect to anything rendered. Only sub-message token deltas (collapsed into the final block) and pure protocol chatter are excluded.
- **Thinking persists here.** Reading what the agent was thinking after the fact is a wanted feature, so `Thinking` is a first-class `ContentBlock`, not audit-log-only. `ToolResult.output` carries rendered content (tool output, diff hunks) so cards redraw without the live stream.
- A permission the user approved persists in compact form (it was part of what they saw); full detail goes to the audit log.
- This is the portable, shareable, fork-seed unit.

### Tier 2: audit log (`events.jsonl`) — local receipts
- The fuller account of what the agent did: permission resolutions in full, full tool args, which downstream MCP servers the gateway hit, command executions, retries.
- This is the trust artifact for the gateway and credential-brokering story: "here is exactly what the agent did and which servers it touched." On-brand, not overhead.
- **Local, not portable.** It never leaves in a `.tamtri` share bundle unless the user explicitly opts in (verbose and potentially sensitive).
- Milestone: reserved in the vault layout in milestone 1 (created empty), written from milestone 3 when events actually flow. A viewer comes later.

### Tier 3: raw ACP wire trace — ephemeral, opt-in debug only
- The literal `session/update` JSON-RPC firehose. Not persisted by default. Capturable behind a flag when debugging. Never part of the durable vault.

### Constraints (non-negotiable)
- **Secrets never persist, even though logs are legible.** The gateway holds and injects credentials; the audit log records "injected credential for server X," never the value. A plain-text log that captured injected keys would be a disaster.
- **Transcript portable, audit log local.** Fork seeds from the transcript. Share bundles carry the transcript and attachments by default, not `events.jsonl`.
- **Legible means unencrypted at rest.** That is the trust model: plain files on your machine, your machine is the boundary. An encrypted vault is a future option, not MVP. Named so it is a decision, not an accident.

---

## 18. Storage artifacts: everything on disk

The single canonical reference for what tamtri persists, where, in what format, and whether it travels. Principle throughout: durable state is plain text in the user's vault (legible, user-owned), secrets are never in the vault, and the transcript is the one portable unit.

### Per-conversation (inside each conversation folder)

```
<vault>/conversations/<yyyy-mm-dd>-<slug>--<shortid>/
  meta.json        conversation metadata
  messages.jsonl   the transcript
  events.jsonl     the audit log (receipts)
  attachments/     curated rendered artifacts
  workdir/         the harness working directory (VaultLocal only)
```

| Artifact | Format | Purpose | Portable? | Written from |
|---|---|---|---|---|
| `meta.json` | JSON, rewritten freely | schema_version, id, title, timestamps, active_harness_id, model_id, working_dir, mcp_servers, roots, forked_from | yes (in bundle) | M1 |
| `messages.jsonl` | JSONL, append-only, one Message/line | the transcript; complete render source; fork seed | yes (the portable unit) | M1 |
| `events.jsonl` | JSONL, append-only | local receipts: permission resolutions, full tool args, downstream servers hit, commands | no (local; opt-in only) | M3 |
| `attachments/` | files + hash refs from transcript | curated artifacts tamtri rendered/snapshotted (e.g. `report.html`) | yes (in bundle) | M6 (reserved M1) |
| `workdir/` | arbitrary files | the harness's cwd for VaultLocal: user inputs + all harness working files | no (messy; not in bundle) | M3 (reserved M1) |

### workdir/ vs attachments/ (important distinction)
- `workdir/` is the harness's filesystem for the conversation. It holds inputs the user provides and everything the harness reads and writes. It can be messy (temp files, a whole repo's worth of state). It persists with the conversation so resuming and re-rendering work.
- `attachments/` is the curated render/share set. When a harness produces a renderable artifact (detected via `FileChanged`), tamtri **snapshots a copy** from workdir into `attachments/`, content-hashed, referenced by a transcript `Artifact` block. This is a real copy step (correcting the earlier "no copy step" claim). It keeps the transcript self-contained and keeps the share bundle to the clean artifacts, not the messy workdir.
- For External working dirs, same idea: the harness's cwd is the external path, and rendered artifacts snapshot into `attachments/`. The external tree never travels.

### Vault-level

| Artifact | Format | Purpose | Notes |
|---|---|---|---|
| vault root | user-chosen folder (e.g. `~/tamtri/`) | contains `conversations/` | syncs via the user's own iCloud/Dropbox/Git |
| app/global config | JSON, legible, vault-level | default harness, picker roster, gateway downstream-server registry, credential *references*, MCP timeouts (`mcp.call_timeout_secs` global default 300s; per-server override on registry entries) | secrets are references only, never values |
| SQLite index (optional) | `index.db` | rebuildable list/search cache | deferred; never a source of truth |

### Shell-side / non-vault (macOS)

| Artifact | Where | Why not the vault |
|---|---|---|
| security-scoped bookmarks | shell storage keyed by conversation id | platform-specific binary; grants durable sandbox-legal access to External dirs |
| UI/window preferences | UserDefaults | inherently shell-side, not portable |
| secrets | OS keychain | never plain text, never in the vault |

### Export format
- `.tamtri` share bundle: a zip of `meta.json` + `messages.jsonl` + `attachments/`. Snapshots artifact bytes and hash-verifies at the bundle boundary. Does **not** include `events.jsonl` or `workdir/` unless the user explicitly opts in.

### Deliberately NOT stored by tamtri
- **Skills.** Harness-owned (they live in the harness's own config, e.g. `~/.claude/`). tamtri reflects them, does not store them.
- **Secrets.** OS keychain only. The vault and both logs hold references, never values.
- **Raw ACP wire traces.** Ephemeral, opt-in debug capture only. Never part of the durable vault.
