# Plan 008: Add root test script and wire app tests into verification baseline

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- package.json CLAUDE.md CONTRIBUTING.md`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

The Synara redesign added meaningful app unit tests (`packages/app/test/*.test.ts`) but the root `package.json` has no `test` script. `CLAUDE.md` tells contributors to run `cargo test`, `clippy`, and `typecheck` — not app tests. Redesign regressions in project tree, settings nav, and conversation surface helpers can ship while the documented checklist stays green.

## Current state

**Root scripts** (`package.json:8-21`): `typecheck`, `build`, dev scripts — no `test`.

**App test script** (`packages/app/package.json:14`):

```json
"test": "node --import tsx --test test/*.test.ts"
```

**Client tests** (`packages/client/package.json:14`): same pattern.

**CLAUDE.md:272-284**:

```
cargo test
cargo clippy --all-targets -- -D warnings
pnpm run typecheck
```

No app test mention.

**Existing app tests**: `project-shell.test.ts`, `settings-navigation.test.ts`, `density.test.ts`, `conversation-surface.test.ts`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| App tests | `pnpm --filter @tamtri/app run test` | exit 0 |
| Client tests | `pnpm --filter @tamtri/client run test` | exit 0 |
| Root test (after) | `pnpm run test` | exit 0, all workspace tests |
| Typecheck | `pnpm run typecheck` | exit 0 |

## Scope

**In scope**:

- `package.json` (root scripts)
- `CLAUDE.md` (Commands section only)
- `CONTRIBUTING.md` (verification checklist, if present and stale)

**Out of scope**:

- Adding Biome/ESLint (DX-01 broader scope).
- New test files (other plans add those).
- CI workflow (DX-02).

## Git workflow

- Branch: `advisor/008-root-test-script`
- Commit message: `Add root test script and document app tests in CLAUDE.md`
- Do NOT push unless instructed.

## Steps

### Step 1: Add root test script

In root `package.json` scripts:

```json
"test": "pnpm -r --if-present run test"
```

This aggregates `@tamtri/app`, `@tamtri/client`, and any other package with a `test` script.

**Verify**: `pnpm run test` → exit 0, output shows app + client tests passing

### Step 2: Update CLAUDE.md Commands section

Add after typecheck line:

```
pnpm run test
```

And optionally:

```
pnpm --filter @tamtri/app run test
```

Keep Rust commands unchanged.

**Verify**: `rg 'pnpm run test' CLAUDE.md` → at least one match in Commands section

### Step 3: Update CONTRIBUTING.md if needed

If `CONTRIBUTING.md:16-19` lists only Rust checks, add `pnpm run test`.

**Verify**: read CONTRIBUTING.md — verification list includes TS tests

## Test plan

- Meta: running `pnpm run test` is the test of this plan.
- Ensure all existing `packages/app/test/*.test.ts` pass without modification.

## Done criteria

- [ ] `pnpm run test` exits 0
- [ ] `pnpm run typecheck` exits 0
- [ ] `CLAUDE.md` documents `pnpm run test`
- [ ] Only in-scope files modified
- [ ] `plans/README.md` row 008 → DONE

## STOP conditions

- Root `test` script already exists — mark DONE after verifying CLAUDE.md.
- `pnpm run test` fails due to pre-existing failing tests — report failing file names; fix only if trivial, else STOP.

## Maintenance notes

- Future plans (001, 003, etc.) should run `pnpm run test` in Done criteria.
- CI addition (DX-02) should call the same root script.
