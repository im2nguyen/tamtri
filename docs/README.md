# tamtri documentation

tamtri is an open-source agent UI shell for **technical-adjacent knowledge workers**: turn data into reports in a real app, not a terminal. A Rust **daemon** owns the vault, MCP gateway, and harness processes; **Electron**, **web**, and **mobile (Expo)** are thin clients over one wire protocol.

## Start here

| Doc | Audience | Contents |
|-----|----------|----------|
| [getting-started.md](./getting-started.md) | Users & evaluators | Install tamtri, set up agent apps, first conversation |
| [product-brief.md](./product-brief.md) | Product / positioning | Problem, thesis, trust model |
| [tamtri-decisions.md](./tamtri-decisions.md) | Contributors | Architectural decisions and rationale |

## Architecture

| Doc | Contents |
|-----|----------|
| [daemon-protocol.md](./daemon-protocol.md) | WebSocket JSON-RPC, methods, credentials |
| [vault-format.md](./vault-format.md) | Legible conversation storage (`messages.jsonl`, attachments) |
| [harness-adapter.md](./harness-adapter.md) | `HarnessAdapter` trait and event vocabulary |
| [provider-adapters.md](./provider-adapters.md) | Claude/Codex/OpenCode/Pi native + ACP roster |
| [mcp-client.md](./mcp-client.md) | Downstream MCP client and dispatch loop |
| [orchestration.md](./orchestration.md) | Recipe engine, async runs, agent MCP tools |
| [renderer.md](./renderer.md) | UI surfaces, artifact preview, sandbox rules |
| [design.md](./design.md) | Canonical visual system and interaction grammar |
| [relay-threat-model.md](./relay-threat-model.md) | Remote access via E2E relay |
| [external-path-brokering.md](./external-path-brokering.md) | External working directories and bookmarks |
| [events-format.md](./events-format.md) | Local audit log (`events.jsonl`) |

## UI development

| Doc | Contents |
|-----|----------|
| [../packages/app/README.md](../packages/app/README.md) | Expo app structure, dev commands |
| [visual-qa-checklist.md](./visual-qa-checklist.md) | Manual UI verification |

## Verification (developers)

| Doc | Contents |
|-----|----------|
| [testing/README.md](./testing/README.md) | Index of feature verification guides |
| [testing/elicitation.md](./testing/elicitation.md) | Form and URL elicitation |
| [testing/oauth.md](./testing/oauth.md) | Remote OAuth connect |
| [testing/apps.md](./testing/apps.md) | MCP Apps |
| [testing/tasks.md](./testing/tasks.md) | Task cards |
| [testing/roots.md](./testing/roots.md) | Filesystem roots |

## Product status

**Shipped today (experimental):**

- Legible vault, fork/import/export, search (titles + text/thinking)
- Harness adapters: ACP long-tail, Claude/Codex/OpenCode/Pi native
- MCP gateway (tools, elicitation, Apps, Tasks, Roots, OAuth)
- Artifact snapshot + inline preview (right sidebar, sandboxed HTML on web)
- Orchestration recipes (background runs, agent MCP tools)
- Agents & providers screen, usage quotas (Codex/Claude), appearance theming
- Surfaces: Electron desktop, web, mobile dev via Expo Go (LAN)

**In progress / gaps:**

- First-run onboarding gate and plain-language setup flow
- Packaged Mac download (signed DMG); dev builds use `pnpm run dev:desktop`
- Relay remote access (client exists; hosted relay + daemon bridge pending)
- Native mobile relay pairing; LAN dev works today
- Some error-state copy and accessibility polish

See [product-gaps.md](./product-gaps.md) for tracked UX gaps.

## Historical build specs

Early build-session specs (formerly `milestone-*.md`) live in [archive/milestones/](./archive/milestones/). They are **implementation history** for contributors tracing how features landed, not the current product map. Prefer this README and the live docs above for what tamtri is **now**.

## Repo map

```
/core              TamtriCore: vault, gateway, harness adapters, protocol
/daemon            tamtri-daemon (WebSocket server)
/packages
  /protocol        Wire types (typeshare)
  /client          DaemonClient SDK
  /relay           E2E relay crypto
  /app             Expo + React Native Web UI
  /desktop         Electron shell
/fixtures          Test/mock binaries (not shipped)
/docs              This directory
```

Agent and contributor instructions: [../CLAUDE.md](../CLAUDE.md).
