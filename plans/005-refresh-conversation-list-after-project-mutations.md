# Plan 005: Refresh conversation list after project mutations

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/hooks/use-projects.ts packages/app/src/hooks/use-conversations.ts`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

Deleting or renaming a project updates vault metadata (`conversation.project_id` cleared to None on delete per `core/src/app.rs:1283`), but the client only invalidates the projects query. The conversation list hook refreshes on harness events (`message_committed`, `turn_started`, `turn_ended`), not project RPCs. The sidebar can show stale `project_id` on thread rows until reload or a harness turn.

## Current state

**Project mutations** (`packages/app/src/hooks/use-projects.ts:45-49`):

```ts
const deleteProject = useCallback(async (id: string) => {
  await client.request(method.PROJECT_DELETE, { id });
  await refreshAll();  // projects query only
}, ...);
```

Same pattern for `renameProject` / `createProject` — no conversation list refresh.

**Conversation list events** (`packages/app/src/hooks/use-conversations.ts:28-37`):

```ts
if (
  event.kind === "message_committed" ||
  event.kind === "turn_started" ||
  event.kind === "turn_ended"
) {
  void refresh();
}
```

**Server behavior** (`core/src/app.rs:1278-1287`): delete rewrites each affected conversation's `project_id` to `None`.

**Tree masking** (`packages/app/src/lib/project-tree.ts:23-28`): orphan conversations bucket into Unfiled when `project_id` missing from project list — partial mitigation, not sufficient for raw summary fields.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |

## Scope

**In scope**:

- `packages/app/src/hooks/use-projects.ts`

**Out of scope**:

- Plan 009 hook deduplication (can land later).
- Daemon event for `project_deleted` (client-side refresh is sufficient).
- `useConversationList` implementation changes (unless needed for shared invalidation helper).

## Git workflow

- Branch: `advisor/005-refresh-conversations-after-project-mutations`
- Commit message: `Refresh conversation list after project mutations`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Import conversation refresh mechanism

Option A (minimal): Accept `refreshConversations` callback parameter on `useProjects` — **avoid**, breaks call sites.

Option B (preferred): Use `queryClient.invalidateQueries` for a shared key, or import a small shared helper.

Option C (simplest): Import `useConversationList` is not valid inside another hook without composition.

**Recommended**: In `use-projects.ts`, use `queryClient.invalidateQueries({ queryKey: ["conversations"] })` if conversations migrate to react-query later; **today** `useConversationList` uses local `useState`, not react-query.

Therefore: export a lightweight **conversation list invalidation bus** or call pattern:

1. Add optional `onProjectsMutated?: () => void` to `useProjects` — **no**, too invasive.

**Pragmatic fix**: Create `packages/app/src/hooks/conversation-list-invalidation.ts`:

```ts
type Listener = () => void;
const listeners = new Set<Listener>();
export function subscribeConversationListInvalidation(fn: Listener) { ... }
export function invalidateConversationList() { listeners.forEach(fn => fn()); }
```

Wire `useConversationList` to subscribe and call `refresh` on invalidation.
Wire `useProjects` `deleteProject`, `renameProject`, `createProject` to call `invalidateConversationList()` after success.

If the codebase already has an event bus pattern, reuse it instead.

**Verify**: `pnpm run typecheck` → exit 0

### Step 2: Invalidate after all project mutations

Call invalidation at end of:

- `deleteProject`
- `renameProject`
- `createProject` (new project may affect tree grouping)
- `attachFilesystemRoot` (optional but good for root counts in sidebar)

**Verify**: `pnpm run typecheck` → exit 0

### Step 3: Add unit test for invalidation bus

Test that `invalidateConversationList` invokes subscribed listeners (pure module test).

**Verify**: `pnpm --filter @tamtri/app run test` → exit 0

## Test plan

- New: `packages/app/test/conversation-list-invalidation.test.ts` (if bus extracted)
- Manual: Delete empty project → sidebar thread rows immediately show under Unfiled without reload

## Done criteria

- [ ] `pnpm run typecheck` exits 0
- [ ] `pnpm --filter @tamtri/app run test` exits 0
- [ ] After `deleteProject`, conversation summaries refetch within same session
- [ ] Only in-scope files modified
- [ ] `plans/README.md` row 005 → DONE

## STOP conditions

- `useConversationList` already refetches on project mutations (already fixed).
- Conversations migrated to react-query with shared invalidation — adapt to query keys instead of bus.

## Maintenance notes

- Plan 006 `moveConversationToProject` must also call invalidation (add in 006 or here with stub).
- Plan 009 may replace this with a single provider; invalidation bus remains valid interim.
