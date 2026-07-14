# Plan 002: Fix project root attach scope payload

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/hooks/use-projects.ts core/src/conversation/model.rs core/src/app.rs core/tests/projects.rs`
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

The redesigned project sidebar exposes "Add shared project root". The client sends `scope: "project"`, but the daemon only accepts `"conversation"` or `"user"`. Every attach attempt fails with `unknown root scope: project`. Shared project roots are a core project feature; the fix is a one-line client correction — project association is expressed via `RootOrigin::Project` on the server, not a scope enum value.

## Current state

**Client bug** (`packages/app/src/hooks/use-projects.ts:60-66`):

```ts
const root = await client.request<RootDto>(method.PROJECT_ROOT_ATTACH, {
  project_id: projectId,
  name,
  uri: path,
  kind: "filesystem",
  scope: "project",  // INVALID
});
```

**Server accepts only** (`core/src/app.rs:4267-4274`):

```rust
fn parse_root_scope(scope: &str) -> Result<RootScope> {
    match scope {
        "conversation" => Ok(RootScope::Conversation),
        "user" => Ok(RootScope::User),
        _ => Err(CoreError::MalformedVault(format!(
            "unknown root scope: {scope}"
        ))),
    }
}
```

**`RootScope` enum** (`core/src/conversation/model.rs:372-376`): `Conversation`, `User` only.

**Canonical test usage** (`core/tests/projects.rs:117-122`):

```rust
core.attach_project_root(
    project.id.clone(),
    "Shared".into(),
    "/tmp/tamtri-shared".into(),
    "filesystem".into(),
    "conversation".into(),  // correct scope for project roots
)
```

**Server sets origin** (`core/src/project.rs:148-155`): `origin: RootOrigin::Project` automatically in `attach_root`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Rust tests | `cargo test` | exit 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |

## Scope

**In scope**:

- `packages/app/src/hooks/use-projects.ts`

**Out of scope**:

- Adding `Project` variant to `RootScope` (unnecessary; tests use `"conversation"`).
- Protocol/types changes.
- UI copy in project sidebar.

## Git workflow

- Branch: `advisor/002-fix-project-root-attach-scope`
- Commit message: `Fix project root attach to use conversation scope`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Fix scope in attachFilesystemRoot

In `packages/app/src/hooks/use-projects.ts`, change `scope: "project"` to `scope: "conversation"` in the `PROJECT_ROOT_ATTACH` payload (line ~65).

**Verify**: `rg 'scope: "project"' packages/app/` → no matches

### Step 2: Optional characterization test

If a convenient place exists, add a small test in `packages/app/test/project-shell.test.ts` or a new test that mocks the client and asserts the attach payload uses `scope: "conversation"`. Skip if mocking the daemon client is disproportionate — the Rust integration test already covers the server path.

**Verify**: `pnpm --filter @tamtri/app run test` → exit 0

## Test plan

- Existing: `core/tests/projects.rs` `effective_roots_propagate_and_dedupe` and `export_materializes_project_roots_and_import_is_unfiled` use `"conversation"` scope.
- Manual: In desktop dev, project sidebar → attach filesystem root → succeeds without daemon error.

## Done criteria

- [ ] `cargo test` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `pnpm run typecheck` exits 0
- [ ] `rg 'scope: "project"' packages/` returns no matches
- [ ] Only `use-projects.ts` modified (unless test added)
- [ ] `plans/README.md` row 002 → DONE

## STOP conditions

- `use-projects.ts` already uses `scope: "conversation"` (already fixed).
- `RootScope` enum gained a `Project` variant and server docs require `scope: "project"` — report and revise approach.
- Attach still fails after scope fix — report daemon error message.

## Maintenance notes

- `scope` on a project root describes user vs conversation visibility semantics; `origin: project` identifies shared project roots.
- If a future `RootScope::Project` is added, update this plan's assumption and tests together.
