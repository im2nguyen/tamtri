# Plan 004: Snapshot project roots onto conversations when deleting a project

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- core/src/app.rs core/src/project.rs core/tests/projects.rs`
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

When a project is deleted, conversations move to Unfiled (`project_id = None` on disk, projected as Unfiled in DTOs). Conversations that relied solely on shared project roots lose those roots on the next harness run because `effective_roots_for_conversation` only merges project roots while `project_id` is set. Export already materializes project roots as `project_snapshot` origins; delete should do the same so live runs and portability stay consistent.

## Current state

**Delete clears project link only** (`core/src/app.rs:1278-1287`):

```rust
for summary in self.vault.list()? {
    if summary.project_id != Some(project_id) { continue; }
    let mut conversation = self.vault.load(summary.id)?;
    conversation.project_id = None;
    conversation.touch();
    self.vault.save_meta(&conversation)?;
    ...
}
self.projects.delete(project_id)
```

**Effective roots without project** (`core/src/app.rs:1249-1258`):

```rust
let Some(project_id) = conversation.project_id else {
    return Ok(conversation.roots.clone());  // project roots gone
};
```

**Export snapshot pattern** (`core/src/vault/bundle.rs:45-54`):

```rust
portable.project_id = None;
portable.roots = effective_roots.into_iter().map(|mut root| {
    if matches!(root.origin, RootOrigin::Project) {
        root.origin = RootOrigin::ProjectSnapshot;
    }
    root
}).collect();
```

**Dedupe helper** (`core/src/project.rs:204-216`): `effective_roots(project_roots, conversation_roots)`.

**Existing test** (`core/tests/projects.rs:164-181`): asserts DTO `project_id` becomes Unfiled after delete; does **not** assert roots preserved.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Rust tests | `cargo test` | exit 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Typecheck | `pnpm run typecheck` | exit 0 |

## Scope

**In scope**:

- `core/src/app.rs` (`delete_project_inner`)
- `core/tests/projects.rs`

**Out of scope**:

- UI delete confirmation copy.
- Blocking delete when project has roots (UI-only guard today).
- Export/import bundle logic (already correct).

## Git workflow

- Branch: `advisor/004-snapshot-roots-on-project-delete`
- Commit message: `Snapshot project roots onto conversations when deleting project`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Load project roots before delete loop

In `delete_project_inner`, after `self.projects.load(project_id)?` and before the conversation loop, capture `let project_roots = self.projects.load(project_id)?.roots.clone()` (or reuse loaded project).

### Step 2: Materialize roots onto each affected conversation

For each conversation with matching `project_id`:

1. Compute `let merged = effective_roots(&project_roots, &conversation.roots)`.
2. Rewrite each root with `origin: RootOrigin::Project` to `RootOrigin::ProjectSnapshot` (match export semantics).
3. Set `conversation.roots = merged`.
4. Set `conversation.project_id = None`.
5. `save_meta` as today.

Do not duplicate roots already present on the conversation ( `effective_roots` dedupes by kind+uri).

**Verify**: `cargo test deleting_project_moves_conversations_to_unfiled` → pass

### Step 3: Add regression test for root preservation

In `core/tests/projects.rs`, add test `deleting_project_preserves_inherited_roots`:

1. Create project with `attach_project_root` (scope `"conversation"`).
2. Create conversation in project **without** conversation-local roots.
3. `list_roots` for conversation → includes shared root.
4. `delete_project`.
5. `list_roots` for conversation → still includes root with `origin: project_snapshot`.
6. Optional: read `meta.json` on disk — `project_id` absent/null, roots array contains snapshot.

**Verify**: `cargo test deleting_project_preserves_inherited_roots` → pass

### Step 4: Full verification

**Verify**: `cargo test` → exit 0; `cargo clippy --all-targets -- -D warnings` → exit 0

## Test plan

- New test: `deleting_project_preserves_inherited_roots` in `core/tests/projects.rs`
- Pattern: `export_materializes_project_roots_and_import_is_unfiled` in same file
- Edge case: conversation with overlapping conversation root + project root → deduped, origins upgraded appropriately

## Done criteria

- [ ] `cargo test` exits 0 including new test
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `pnpm run typecheck` exits 0
- [ ] Only `core/src/app.rs` and `core/tests/projects.rs` modified
- [ ] `plans/README.md` row 004 → DONE

## STOP conditions

- `delete_project_inner` signature or location changed substantially.
- `RootOrigin` enum changed — verify `ProjectSnapshot` still exists.
- Merged roots break `validate_root` — report which root shape fails.

## Maintenance notes

- Matches export portability story in `CLAUDE.md`: inherited roots become snapshots when project link breaks.
- Reviewers should check meta.json legibility after delete (roots visible, project_id null).
- Performance: delete still O(conversations) loads — PERF-14 is separate backlog.
