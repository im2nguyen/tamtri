# Plan 009: Dedupe conversation list hook and reduce vault RPC fan-out

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/hooks/use-conversations.ts packages/app/src/components/sidebar/left-sidebar.tsx packages/app/src/components/layout/home-pane.tsx packages/app/src/hooks/use-onboarding-gate.ts packages/app/src/runtime/`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW
- **Depends on**: plans/005-refresh-conversation-list-after-project-mutations.md (invalidation bus should integrate with provider)
- **Category**: perf
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

Each `useConversationList()` instance fetches `CONVERSATION_LIST` on mount and subscribes to daemon events. Three instances mount on `/` (left sidebar, home pane, onboarding gate), tripling startup RPCs and event-driven vault scans. Additionally, `turn_started` and `turn_ended` trigger full list refreshes even though summaries do not change until `message_committed`.

## Current state

**Hook** (`packages/app/src/hooks/use-conversations.ts:6-40`):

- `useState` + `refresh()` on mount.
- Subscribes to `message_committed`, `turn_started`, `turn_ended` → all call `refresh()`.

**Consumers**:

- `packages/app/src/components/sidebar/left-sidebar.tsx:42`
- `packages/app/src/components/layout/home-pane.tsx:37`
- `packages/app/src/hooks/use-onboarding-gate.ts:40` (via onboarding router tree)

**Server cost**: `CONVERSATION_LIST` scans vault (`core/src/vault/fs.rs` via `scan_entries`).

**Invalidation** (from plan 005): `invalidateConversationList()` bus for project mutations.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |
| Root test | `pnpm run test` | exit 0 |

## Scope

**In scope**:

- `packages/app/src/hooks/use-conversations.ts` (refactor)
- `packages/app/src/runtime/conversation-list-provider.tsx` (create)
- `packages/app/src/app/_layout.tsx` (wrap provider)
- `packages/app/src/hooks/use-onboarding-gate.ts` (switch to context hook)
- `packages/app/src/components/sidebar/left-sidebar.tsx` (no API change if hook stable)
- `packages/app/src/components/layout/home-pane.tsx` (same)
- `packages/app/test/conversation-list-provider.test.ts` (create, optional)

**Out of scope**:

- Server-side list caching or pagination.
- PERF-03 sendMessage full reload.
- React Query migration for all daemon data.

## Git workflow

- Branch: `advisor/009-dedupe-conversation-list`
- Commit message: `Share conversation list state via provider and trim refresh events`
- Do NOT push unless instructed.

## Steps

### Step 1: Create ConversationListProvider

Create `packages/app/src/runtime/conversation-list-provider.tsx`:

- Single `useEffect` mount fetch for `CONVERSATION_LIST`.
- Single daemon event subscription.
- Expose `{ conversations, loading, error, refresh }` via React context.
- Integrate `subscribeConversationListInvalidation` from plan 005 (call `refresh` on invalidate).

Export `useConversationList()` from this module (same signature as today) so call sites need not change imports if you re-export from `hooks/use-conversations.ts` as a thin re-export.

**Verify**: `pnpm run typecheck` → exit 0

### Step 2: Mount provider once in app shell

In `packages/app/src/app/_layout.tsx`, wrap children with `ConversationListProvider` inside `DaemonProvider` (needs client access).

**Verify**: App boots without runtime error (manual or existing smoke script if available)

### Step 3: Trim event-driven refresh

In provider subscription, refresh only on:

- `message_committed` (title or metadata may change)
- Optionally `conversation_created` if such event exists — grep protocol; if none, keep project invalidation bus only.

**Remove** `turn_started` and `turn_ended` from refresh triggers.

Rationale: conversation summaries for list view do not change on turn boundaries; active conversation pane uses its own hook.

**Verify**: `rg 'turn_started|turn_ended' packages/app/src/runtime/conversation-list-provider.tsx packages/app/src/hooks/use-conversations.ts` → no matches in refresh logic

### Step 4: Remove duplicate hook instances

Ensure `use-onboarding-gate.ts` uses context `useConversationList` (import path unchanged if re-exported).

Delete old standalone fetch logic from deprecated hook file if fully moved.

**Verify**: `rg 'useConversationList' packages/app/src` → all imports resolve; only one `CONVERSATION_LIST` subscription in provider (grep)

### Step 5: Tests

Add test that provider module exports stable API; optional test that invalidation bus triggers refresh callback.

**Verify**: `pnpm run test` → exit 0

## Test plan

- Existing app tests must pass (project tree tests use static data, unaffected).
- Manual: Open `/` with dev tools network or daemon logs — one `conversation.list` on load, not three.
- Manual: During harness run, list RPC not fired on every turn start/end.

## Done criteria

- [ ] `pnpm run typecheck` exits 0
- [ ] `pnpm run test` exits 0
- [ ] Single conversation list fetch on home mount (verified manually or via debug log)
- [ ] `turn_started`/`turn_ended` do not trigger list refresh
- [ ] Project mutation invalidation still refreshes list (plan 005)
- [ ] `plans/README.md` row 009 → DONE

## STOP conditions

- App already uses a shared provider — compare and mark DONE if equivalent.
- Removing turn event refresh breaks sidebar title updates — document which event is needed and restore minimal set.
- `_layout.tsx` structure incompatible with provider — report file excerpt.

## Maintenance notes

- If react-query adoption lands repo-wide, provider can be replaced with query cache — keep invalidation contract.
- Reviewers watch for memory leaks (duplicate subscriptions on hot reload).
