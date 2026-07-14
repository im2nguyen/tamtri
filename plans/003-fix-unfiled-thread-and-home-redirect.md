# Plan 003: Fix Unfiled thread creation and home redirect behavior

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/components/layout/home-pane.tsx packages/app/src/components/sidebar/project-sidebar.tsx packages/app/src/lib/project-tree.ts packages/app/test/`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

The Unfiled project holds legacy and onboarding threads. The daemon supports `PROJECT_CONVERSATION_CREATE` with `UNFILED_PROJECT_ID`, but the redesigned home pane excludes Unfiled from `selectedProject` and aborts send without a project. The sidebar disables "New thread" on Unfiled. Separately, home auto-redirects every returning user to their latest conversation, blocking explicit navigation to `/` for a new thread unless they used `beginProjectDraft` from the sidebar.

## Current state

**Unfiled constant** (`packages/app/src/lib/project-tree.ts:3`):

```ts
export const UNFILED_PROJECT_ID = "74616d74-7269-7000-0000-000000000001";
```

**Home excludes Unfiled** (`home-pane.tsx:114-118`):

```ts
const selectedProject =
  projects.find(
    (project) =>
      project.id === selectedProjectId && project.id !== UNFILED_PROJECT_ID,
  ) ?? null;
```

**Send blocked** (`home-pane.tsx:229-232`):

```ts
if (!selectedProject) {
  setError("Create or select a project before starting a thread.");
  return;
}
```

**Sidebar disables Unfiled new thread** (`project-sidebar.tsx:72-79`):

```ts
onNewThread={
  node.isUnfiled
    ? undefined
    : () => { beginProjectDraft(node.id); router.push("/"); ... }
}
```

**Auto-redirect** (`home-pane.tsx:120-137`):

```ts
restorationAttempted.current = true;
// ... valid draft check ...
const latest = conversations[0];
if (latest) {
  // sets selected project from latest
  router.replace(`/conversation/${latest.id}`);
  return;
}
```

Runs unconditionally on first load when conversations exist and no valid `draftProjectId`.

**Daemon supports Unfiled create** (`core/src/app.rs:1339-1342`):

```rust
conversation.project_id = (project_id != unfiled_project_id()).then_some(project_id);
```

**Design** (`docs/design.md`): projects are organizational containers; Unfiled is the stable bucket for unassigned threads.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |
| Rust (unchanged) | `cargo test` | exit 0 |

## Scope

**In scope**:

- `packages/app/src/components/layout/home-pane.tsx`
- `packages/app/src/components/sidebar/project-sidebar.tsx`
- `packages/app/test/home-pane-restoration.test.ts` (create)

**Out of scope**:

- Onboarding starter flow (`starter-screen.tsx` uses legacy `CONVERSATION_CREATE` — works today).
- Changing Unfiled immutability rules in core.
- Project tree ordering (`buildProjectTree`).

## Git workflow

- Branch: `advisor/003-unfiled-thread-home-redirect`
- Commit message: `Allow Unfiled thread creation and fix home redirect`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Treat Unfiled as valid compose target on home

In `home-pane.tsx`:

1. Change `selectedProject` to resolve Unfiled when `selectedProjectId === UNFILED_PROJECT_ID` (look up project in `projects` array without excluding Unfiled id).
2. Alternatively introduce `composeProject` that includes Unfiled and use it in `handleSend` and composer subtitle.
3. Update error copy if no project selected at all (empty projects list edge case).

`handleSend` should call `createConversation(UNFILED_PROJECT_ID, ...)` when Unfiled is selected.

**Verify**: `pnpm run typecheck` → exit 0

### Step 2: Enable New thread on Unfiled in sidebar

In `project-sidebar.tsx`, set `onNewThread` for Unfiled nodes:

```ts
onNewThread={() => {
  beginProjectDraft(UNFILED_PROJECT_ID); // or node.id when isUnfiled
  router.push("/");
  onClose?.();
}}
```

Import `UNFILED_PROJECT_ID` from `@/lib/project-tree` if using constant directly.

**Verify**: `rg 'node.isUnfiled\s*\?\s*undefined' packages/app/src/components/sidebar/project-sidebar.tsx` → no match for `onNewThread`

### Step 3: Gate home auto-redirect

In `home-pane.tsx` restoration effect (`120-156`):

- **Do** auto-redirect when restoring a session is intended: e.g. app cold-start with no explicit `/` navigation intent.
- **Do not** redirect when:
  - `draftProjectId` is set (including Unfiled draft via `beginProjectDraft`), OR
  - user explicitly navigated to home (use a ref set by route focus / `usePathname() === "/"` with `draftProjectId` or a new `useUiStore` flag `wantsHomeComposer`).

Recommended approach: skip `router.replace` when `draftProjectId !== null` (valid or not). Only redirect to latest conversation when `draftProjectId === null` **and** no `wantsHomeComposer` flag. Set `wantsHomeComposer` true when sidebar calls `beginProjectDraft` or user presses a "New thread" affordance; clear on successful send or navigation away.

Minimal fix acceptable: if `draftProjectId !== null`, never auto-redirect; if user opens `/` without draft, still redirect (preserves returning-user default). Document behavior in test.

**Verify**: `pnpm run typecheck` → exit 0

### Step 4: Add restoration tests

Create `packages/app/test/home-pane-restoration.test.ts` testing extracted pure logic if you extract a `resolveHomeRestoration({ draftProjectId, conversations, ... })` helper; otherwise test `beginProjectDraft` + store interactions.

Minimum cases:

- Valid `draftProjectId` (including `UNFILED_PROJECT_ID`) → no redirect to latest conversation.
- `draftProjectId === null` and conversations exist → redirect target is latest id (if logic extracted).
- Unfiled project id is valid for `createConversation` payload.

**Verify**: `pnpm --filter @tamtri/app run test` → all pass

## Test plan

- New: `packages/app/test/home-pane-restoration.test.ts`
- Pattern: `packages/app/test/project-shell.test.ts` (pure helpers + store constants)
- Manual: Sidebar Unfiled → New thread → stays on `/` with composer; send creates thread under Unfiled.

## Done criteria

- [ ] `pnpm run typecheck` exits 0
- [ ] `pnpm --filter @tamtri/app run test` exits 0
- [ ] Unfiled new thread works from sidebar and home send path
- [ ] `beginProjectDraft` + navigate `/` does not bounce to latest conversation
- [ ] Only in-scope files modified
- [ ] `plans/README.md` row 003 → DONE

## STOP conditions

- Unfiled project id constant changed in `project-tree.ts` — update plan references.
- Product decision: Unfiled must not accept new threads — report (contradicts daemon capability).
- Restoration effect structure completely rewritten — re-read and adapt steps.

## Maintenance notes

- Onboarding still seeds Unfiled via legacy path; this plan aligns manual creation with that behavior.
- Reviewers should verify redirect does not loop with `OnboardingRouter`.
- Follow-up: integrate with TEST-03 broader home-pane integration tests.
