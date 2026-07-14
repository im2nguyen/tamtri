# Audit index: Synara-style redesign branch

**Audit commit:** `f9e70ab`  
**Date:** 2026-07-13  
**Scope:** Uncommitted Synara-style redesign (projects, sidebar/home routing, settings, conversation surface, right dock, density/theme)

This file consolidates subagent findings. Implementation steps live in numbered plans `001`–`009`; execution index in [README.md](README.md).

---

## Correctness (8 findings)

| ID | Origin | Summary | Evidence |
|----|--------|---------|----------|
| CORRECTNESS-01 | introduced | Project root attach sends `scope: "project"`; daemon rejects | `use-projects.ts:65`, `app.rs:4267-4274` |
| CORRECTNESS-02 | introduced | Home/sidebar block Unfiled thread creation | `home-pane.tsx:114-118,229-232`, `project-sidebar.tsx:72-79` |
| CORRECTNESS-03 | introduced | Home auto-redirects to latest conversation | `home-pane.tsx:120-137` |
| CORRECTNESS-04 | introduced | Stale conversation list after project delete | `use-projects.ts:45-49`, `use-conversations.ts:28-37` |
| CORRECTNESS-05 | introduced | Project delete drops inherited roots from live runs | `app.rs:1278-1287`, `app.rs:1249-1258` |
| CORRECTNESS-06 | introduced | Corrupt project folders silently skipped in list | `project.rs:76-86` |
| CORRECTNESS-07 | introduced | Model picker active when runtime switch unsupported | `conversation-pane.tsx:704-706`, `composer-controls.tsx:41-42` |
| CORRECTNESS-08 | introduced | Failed send after create orphans empty conversations | `home-pane.tsx:233-255` |

---

## Security (10 findings)

| ID | Origin | Summary | Evidence |
|----|--------|---------|----------|
| SECURITY-01 | introduced | Artifact iframe `allow-same-origin` weakens sandbox | `sandboxed-html.web.tsx:70` |
| SECURITY-02 | introduced | Partial HTML sanitizer; foreign CSP/scripts possible | `sandboxed-html.web.tsx:17-41` |
| SECURITY-03 | introduced | Mobile daemon bearer token in AsyncStorage | `connection-config.ts` |
| SECURITY-04 | pre-existing | Single bearer token = full daemon authority | `daemon/src/server.rs:71-72` |
| SECURITY-05 | pre-existing | Token in WebSocket query string | `websocket-transport.ts:29-31` |
| SECURITY-06 | introduced | LAN `0.0.0.0` bind in iOS dev loop | `scripts/dev-ios.mjs` |
| SECURITY-07 | introduced | Export materializes project root paths into bundle | `bundle.rs:45-54`, `projects.rs:185-219` |
| SECURITY-08 | pre-existing | MCP Apps summary-only (no sandboxed panel) | `message-list.tsx`, `right-dock.tsx` |
| SECURITY-09 | pre-existing | `roots.sync_runtime` URI override without bookmark proof | `app.rs:2320-2337` |
| SECURITY-10 | pre-existing | RPC validation is serde-only | `daemon/dispatch.rs:31-34` |

---

## Performance (15 findings)

| ID | Origin | Summary |
|----|--------|---------|
| PERF-01 | introduced | Duplicate `useConversationList` (3× RPC on home) |
| PERF-02 | introduced | List refresh on `turn_started` / `turn_ended` |
| PERF-03 | introduced | `sendMessage` full transcript reload |
| PERF-04 | introduced | Sidebar search O(conversations) vault loads |
| PERF-05 | introduced | `useVaultIssues` full vault scan on sidebar mount |
| PERF-06 | introduced | Transcript not virtualized |
| PERF-07 | introduced | `JSON.stringify` tool payloads every render |
| PERF-08 | introduced | Right-dock double message scan |
| PERF-09 | introduced | Auto-follow scroll on every content-size change |
| PERF-10 | introduced | Theme CSS universal selectors |
| PERF-11 | introduced | Project `resolve()` O(projects) directory scan |
| PERF-12 | introduced | Duplicate harness provider list fetch |
| PERF-13 | pre-existing | Conversation folder resolve O(conversations) |
| PERF-14 | introduced | Project delete loads every matching conversation |
| PERF-15 | introduced | Turn duration O(N²) per render |

---

## Tests (21 findings)

| ID | Origin | Summary |
|----|--------|---------|
| TEST-01 | introduced | App tests not in root verification baseline |
| TEST-02 | introduced | App tests cover helpers, not shell integration |
| TEST-03 | introduced | Home restoration logic untested |
| TEST-04 | introduced | UI store migration omits `draftProjectId` |
| TEST-05 | introduced | Project-sidebar delete/rename guards untested |
| TEST-06 | introduced | `projectsSupported` gate untested |
| TEST-07 | introduced | Daemon dispatch parity excludes project RPCs |
| TEST-08 | pre-existing | Client E2E lacks project round-trip |
| TEST-09 | introduced | Project RPC tests omit update/delete/root RPCs |
| TEST-10 | introduced | Negative tests for Unfiled immutability missing |
| TEST-11 | introduced | Delete meta vs DTO projection not characterized |
| TEST-12 | introduced | Delete-with-roots UI guard untested |
| TEST-13 | introduced | Onboarding gate/router zero tests |
| TEST-14 | introduced | Settings redirects/scroll targets untested |
| TEST-15 | introduced | Settings search index parity unchecked |
| TEST-16 | introduced | Transcript search scope messaging untested |
| TEST-17 | introduced | Auto-follow grace window untested |
| TEST-18 | introduced | Receipt tests mismatch UI behavior |
| TEST-19 | introduced | Right-dock test decouples artifact count |
| TEST-20 | introduced | `buildProjectTree` ordering contract untested |
| TEST-21 | introduced | Baseline: existing pure-function tests listed |

---

## Architecture / tech debt (16 findings)

| ID | Origin | Summary |
|----|--------|---------|
| TECH-DEBT-01 | introduced | Parallel live-streaming pipelines |
| TECH-DEBT-02 | introduced | Dead `artifact-sidebar` module |
| TECH-DEBT-03 | pre-existing | Orphaned `conversation-row` |
| TECH-DEBT-04 | introduced | God-component `conversation-pane` |
| TECH-DEBT-05 | introduced | `TamtriCore` app.rs too large |
| TECH-DEBT-06 | introduced | Hand-maintained transcript DTOs |
| TECH-DEBT-07 | introduced | Client UiEvent reducer duplicates core |
| TECH-DEBT-08 | introduced | UIMessage round-trips through ContentBlock |
| TECH-DEBT-09 | introduced | Duplicate conversation cache |
| TECH-DEBT-10 | introduced | Inconsistent sidebar row styles |
| TECH-DEBT-11 | introduced | Unsafe UI store migration cast |
| DEPS-01 | introduced | pnpm lockfile untracked |
| DEPS-02 | introduced | TypeScript version skew |
| DEPS-03 | pre-existing | Protocol typeshare gaps |
| DEPS-04 | introduced | Dev token in web-served `public/` |
| TECH-DEBT-12 | pre-existing | Schema v4 serde defaults (by-design) |

---

## Docs / DX (9 findings + 2 direction items)

| ID | Origin | Summary |
|----|--------|---------|
| DOCS-01 | introduced | `/health` → Advanced, not Providers |
| DOCS-02 | introduced | User guides describe pre-project shell |
| DOCS-03 | introduced | Mixed Agents & providers / Providers labels |
| DOCS-04 | introduced | Dead `NewConversationSheet` |
| DOCS-05 | introduced | `CLAUDE.md` omits `design.md` reference |
| DX-01 | pre-existing | No root test/lint scripts |
| DX-02 | pre-existing | No CI workflow |
| DX-03 | pre-existing | Stale `packages/app/README.md` |
| DOCS-06 | pre-existing | `product-gaps.md` onboarding contradiction |
| DOCS-07 | introduced | Renderer missing move_project + root_remove |
| DOCS-08 | pre-existing | `docs/README.md` "right sidebar" vs right dock |
| DX-04 | pre-existing | No `.env.example` |
| DOCS-09 | introduced | Visual QA dev commands differ from README |

### Direction

1. Finish project-container UX (move thread, remove root) — plan 006.
2. Canon settings + vocabulary pass (`/health`, Providers naming, user docs) — backlog.

---

## Verified by-design (no finding)

- Legible unencrypted vault intentional.
- Import hash verification intact.
- Attachment path traversal guards intact.
- Renderer does not bypass daemon for vault/gateway.
- Root `project_snapshot` on export matches `vault-format.md`.
