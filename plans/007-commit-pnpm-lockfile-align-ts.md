# Plan 007: Commit pnpm lockfile and align TypeScript versions

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- package.json pnpm-lock.yaml pnpm-workspace.yaml packages/*/package.json`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: deps
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

The repo migrated to pnpm (`packageManager: pnpm@9.12.3`) but `pnpm-lock.yaml` is untracked and `package-lock.json` is deleted in the working tree. Clean clones cannot reproduce the dependency graph. TypeScript versions differ: app uses `~5.9.2`, other workspaces use `^5.8.3`, causing cross-package typecheck drift.

## Current state

**Root** (`package.json:7-8,23-24`):

```json
"packageManager": "pnpm@9.12.3",
"devDependencies": { "typescript": "^5.8.3" }
```

**App** (`packages/app/package.json:47`):

```json
"typescript": "~5.9.2"
```

**Client** (`packages/client/package.json:24`):

```json
"typescript": "^5.8.3"
```

**Git status at audit**: `D package-lock.json`, `?? pnpm-lock.yaml`

**Verification baseline** (`CLAUDE.md:272-284`): `cargo test`, `clippy`, `pnpm run typecheck`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Install | `pnpm install` | exit 0, lockfile consistent |
| Typecheck | `pnpm run typecheck` | exit 0 |
| Rust | `cargo test` | exit 0 |

## Scope

**In scope**:

- `pnpm-lock.yaml` (add to git)
- `package-lock.json` (ensure removed from git)
- `package.json` (root — TS override)
- `packages/app/package.json`
- `packages/client/package.json`
- `packages/protocol/package.json`
- `packages/desktop/package.json`
- `packages/relay/package.json` (if has typescript devDep)
- `CONTRIBUTING.md` (only if it references npm install — optional one-line fix)

**Out of scope**:

- Upgrading Expo/React versions.
- CI workflow (DX-02 backlog).
- `node_modules` contents.

## Git workflow

- Branch: `advisor/007-pnpm-lockfile-align-ts`
- Commit message: `Commit pnpm lockfile and align TypeScript versions`
- Do NOT push unless instructed.

## Steps

### Step 1: Align TypeScript to single version

At root `package.json`, add under `"pnpm"`:

```json
"overrides": {
  "typescript": "5.9.3"
}
```

(Merge with existing react overrides if present — keep both keys in one `overrides` object.)

Remove per-workspace `typescript` devDependencies or set all to `"5.9.3"` exact — prefer root override so workspaces inherit.

**Verify**: `pnpm run typecheck` → exit 0 after install

### Step 2: Regenerate and commit lockfile

```bash
pnpm install
git add pnpm-lock.yaml
git rm package-lock.json  # if still tracked
```

Ensure `pnpm-workspace.yaml` is tracked (should already be).

**Verify**: `test -f pnpm-lock.yaml && ! test -f package-lock.json` → true in repo root after commit staging

### Step 3: Document install path

If `CONTRIBUTING.md` or `README.md` still says `npm install`, update to `pnpm install` (single line only if wrong).

**Verify**: `rg 'npm install' README.md CONTRIBUTING.md` → no matches unless intentional legacy note

### Step 4: Full verification

**Verify**: `pnpm run typecheck` → exit 0; `cargo test` → exit 0

## Test plan

- No new unit tests; verification is reproducible install + typecheck.
- Clone simulation: `pnpm install --frozen-lockfile` in clean worktree if operator wants extra confidence.

## Done criteria

- [ ] `pnpm-lock.yaml` tracked in git
- [ ] `package-lock.json` not tracked
- [ ] `pnpm run typecheck` exits 0
- [ ] `cargo test` exits 0
- [ ] All workspaces resolve same TypeScript version (`pnpm why typescript` shows single version)
- [ ] `plans/README.md` row 007 → DONE

## STOP conditions

- `pnpm-lock.yaml` already committed and package-lock already gone — mark DONE.
- `pnpm install` fails with resolution errors — report full error.
- TypeScript 5.9 breaks typecheck in a package — report package name and error count.

## Maintenance notes

- Future dependency bumps: always commit lockfile changes with the bump commit.
- Reviewers verify no secrets in lockfile or env files.
