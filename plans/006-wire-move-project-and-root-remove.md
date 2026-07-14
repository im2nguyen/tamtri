# Plan 006: Wire move_project and root_remove in the renderer

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/hooks/use-projects.ts packages/app/src/components/sidebar/ packages/protocol/src/methods.ts`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED
- **Depends on**: plans/005-refresh-conversation-list-after-project-mutations.md
- **Category**: dx
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

The daemon and protocol expose `conversation.move_project` and `project.root_remove`, documented in `docs/daemon-protocol.md`. The redesigned shell implements create/rename/delete/attach only. Knowledge-work users cannot move threads between projects or remove shared roots without another client — undermining the project-container UX direction.

## Current state

**Protocol methods** (`packages/protocol/src/methods.ts`):

```ts
CONVERSATION_MOVE_PROJECT: "conversation.move_project",
PROJECT_ROOT_REMOVE: "project.root_remove",
```

**Hook surface** (`packages/app/src/hooks/use-projects.ts:27-81`): `createProject`, `renameProject`, `deleteProject`, `attachFilesystemRoot`, `createConversation` — no move or root remove.

**Server** (`core/src/app.rs:1309-1329`): `move_conversation_to_project_inner`, `remove_project_root_inner` implemented.

**Docs gap** (`docs/daemon-protocol.md:70`): documents move; renderer gap confirmed by grep — no `MOVE_PROJECT` / `ROOT_REMOVE` in `packages/app`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |
| Rust tests | `cargo test` | exit 0 |

## Scope

**In scope**:

- `packages/app/src/hooks/use-projects.ts`
- `packages/app/src/components/sidebar/project-sidebar.tsx`
- `packages/app/src/components/sidebar/thread-row.tsx` (move action UI)
- `packages/app/src/components/sidebar/project-row.tsx` (root remove if roots listed)
- `packages/app/src/components/sidebar/left-sidebar.tsx` (wire handlers)
- `packages/app/test/project-mutations.test.ts` (create, optional)

**Out of scope**:

- Drag-and-drop between projects (use context menu or row action first).
- Daemon/protocol changes.
- Bulk move.

## Git workflow

- Branch: `advisor/006-wire-move-project-root-remove`
- Commit message: `Add move thread and remove project root actions in sidebar`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Extend useProjects hook

Add:

```ts
const moveConversationToProject = useCallback(
  async (conversationId: string, projectId: string) => {
    const dto = await client.request<ConversationDto>(method.CONVERSATION_MOVE_PROJECT, {
      conversation_id: conversationId,
      project_id: projectId,
    });
    await refreshAll();
    invalidateConversationList(); // from plan 005
    return dto;
  },
  [...],
);

const removeProjectRoot = useCallback(
  async (projectId: string, rootId: string) => {
    await client.request(method.PROJECT_ROOT_REMOVE, { project_id: projectId, root_id: rootId });
    await refreshAll();
  },
  [...],
);
```

Export both from hook return value. Verify param names against `core/src/protocol/params.rs` (read file for exact snake_case keys).

**Verify**: `pnpm run typecheck` → exit 0

### Step 2: Move thread UI

On `ThreadRow`, add overflow action "Move to project…" opening a simple picker (modal listing projects excluding current, plus Unfiled). On select, call `moveConversationToProject`.

Wire through `ProjectSidebar` → `LeftSidebar` props: `onMoveConversation`.

**Verify**: `pnpm run typecheck` → exit 0

### Step 3: Remove root UI

On `ProjectRow` (when expanded, roots visible in `node.project?.roots`), add remove control per root calling `removeProjectRoot`. Guard: not on Unfiled (`node.isUnfiled`).

**Verify**: `pnpm run typecheck` → exit 0

### Step 4: Tests

Add test asserting hook builds correct RPC method names / payload keys (mock client or pure constant test). Optional manual QA checklist in PR.

**Verify**: `pnpm --filter @tamtri/app run test` → exit 0

## Test plan

- Hook payload shape test in `packages/app/test/project-mutations.test.ts`
- Manual: Move thread from Project A to Unfiled → appears under Unfiled without reload
- Manual: Remove shared root → project `roots` length decreases in sidebar

## Done criteria

- [ ] `pnpm run typecheck` exits 0
- [ ] `pnpm --filter @tamtri/app run test` exits 0
- [ ] `cargo test` exits 0
- [ ] Move and remove actions callable from sidebar
- [ ] Conversation list refreshes after move (plan 005)
- [ ] `plans/README.md` row 006 → DONE

## STOP conditions

- Protocol param names differ from assumption — read `params.rs` and adjust.
- `projectsSupported` false — UI should already gate; report if not.
- Move to same project errors on server — handle gracefully in UI.

## Maintenance notes

- Aligns with direction item 1 in `plans/README.md`.
- Reviewers check Unfiled move uses `UNFILED_PROJECT_ID` consistently with server.
- Future: add `core/tests/projects.rs` coverage for `PROJECT_ROOT_REMOVE` RPC (TEST-09 backlog).
