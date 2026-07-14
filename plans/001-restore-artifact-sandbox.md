# Plan 001: Restore artifact sandbox isolation and harden HTML sanitizer

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat f9e70ab..HEAD -- packages/app/src/components/transcript/sandboxed-html.web.tsx packages/app/src/components/artifact/artifact-preview-panel.tsx packages/app/test/`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `f9e70ab`, 2026-07-13

## Why this matters

Harness-produced HTML renders in a sandboxed iframe. The redesign added `sandbox="allow-same-origin"` so the parent page could attach click listeners inside the iframe for blocked-link telemetry. That weakens origin isolation: a parser gap or browser sandbox bug could let artifact HTML access the host shell (including dev daemon credentials in memory and `localStorage`). Artifacts must stay scriptless and networkless per product rules. Restoring strict sandbox plus a stronger sanitizer closes the regression without abandoning blocked-link UX entirely.

## Current state

- `packages/app/src/components/transcript/sandboxed-html.web.tsx` — web-only artifact/App HTML preview; used by transcript and right dock.
- `packages/app/src/components/artifact/artifact-preview-panel.tsx:138-148` — passes `onNavigationBlocked` to log blocked URLs via daemon RPC.

**Sandbox regression** (`sandboxed-html.web.tsx:66-87`):

```tsx
<iframe
  title={title ?? "Artifact preview"}
  src={src}
  sandbox="allow-same-origin"
  ...
  onLoad={(event) => {
    const frame = event.currentTarget;
    try {
      frame.contentWindow?.addEventListener("click", ...);
    } catch { ... }
  }}
/>
```

**Partial sanitizer** (`sandboxed-html.web.tsx:17-41`):

- Strips `http-equiv="refresh"`, external `src`/`href`, injects `ARTIFACT_CSP`.
- Does **not** remove `<script>`, `<iframe>`, `<embed>`, `<object>`, or attacker `<meta http-equiv="Content-Security-Policy">` nodes.

**Design constraints** (from `CLAUDE.md` golden rules):

- "Never give a rendered artifact network access."
- "Do not render harness-produced HTML or MCP App HTML outside the sandboxed webview + consent path."
- Artifacts: `connect-src 'none'`, no scripts on outer iframe.

## Commands you will need

| Purpose   | Command | Expected on success |
|-----------|---------|---------------------|
| Rust tests | `cargo test` | exit 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Typecheck | `pnpm run typecheck` | exit 0 |
| App tests | `pnpm --filter @tamtri/app run test` | exit 0, new tests pass |

## Scope

**In scope**:

- `packages/app/src/components/transcript/sandboxed-html.web.tsx`
- `packages/app/src/components/artifact/artifact-preview-panel.tsx` (only if blocked-link UX changes)
- `packages/app/test/sandboxed-html.test.ts` (create)

**Out of scope**:

- MCP App interactive webview (not implemented yet; see SECURITY-08 in audit).
- Electron-specific webview paths (none exist for artifacts today).
- Daemon `ARTIFACT_LOG_NAVIGATION_BLOCKED` RPC shape.

## Git workflow

- Branch: `advisor/001-restore-artifact-sandbox`
- Commit message style: sentence-case imperative, e.g. `Restore strict artifact iframe sandbox and harden HTML sanitizer`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Restore strict iframe sandbox

In `sandboxed-html.web.tsx`, change the iframe `sandbox` attribute to the strictest setting: empty string `sandbox=""` (no flags). Remove the `onLoad` handler that accesses `frame.contentWindow` — it cannot work without `allow-same-origin` and must not be reintroduced.

**Verify**: `rg 'allow-same-origin' packages/app/src/components/transcript/sandboxed-html.web.tsx` → no matches

### Step 2: Harden `prepareArtifactHtml`

After `DOMParser` parse, before injecting tamtri CSP:

1. Remove all elements matching: `script`, `iframe`, `embed`, `object`, `frame`, `frameset`.
2. Remove all `meta[http-equiv="Content-Security-Policy" i]` and `meta[http-equiv="refresh" i]` (refresh already removed; extend to foreign CSP).
3. Keep existing external `src`/`href` stripping and `data-tamtri-blocked-href` marking on anchors.
4. Prepend tamtri `ARTIFACT_CSP` meta as today.

Export a helper `collectBlockedHrefs(html: string): string[]` that returns unique `data-tamtri-blocked-href` values from the prepared document (parse in parent context before blob creation).

**Verify**: `pnpm --filter @tamtri/app run test -- --test-name-pattern 'sandboxed-html'` → passes (after Step 4 adds tests)

### Step 3: Surface blocked links without iframe DOM access

In `SandboxedHtml`, use `useMemo` to compute `blockedHrefs` from `collectBlockedHrefs(html)`. Render a small native footer below the iframe (React Native `View`/`Text`) listing blocked URLs when non-empty. Each row is pressable and calls `onNavigationBlocked?.(url)` — preserves audit logging without cross-origin iframe access.

Update `artifact-preview-panel.tsx` only if prop wiring changes; keep existing `ARTIFACT_LOG_NAVIGATION_BLOCKED` call.

**Verify**: `pnpm run typecheck` → exit 0

### Step 4: Add regression tests

Create `packages/app/test/sandboxed-html.test.ts` using the repo's existing pattern (`node --import tsx --test`). Test `prepareArtifactHtml` / `collectBlockedHrefs` via exported pure functions (export them from the module or a sibling `sandboxed-html-prep.ts` if web-only DOM APIs need `isWeb` guard — prefer extracting prep logic to a `.web.ts` testable module).

Cases:

- `<script>alert(1)</script>` removed from output HTML.
- Nested `<iframe src="https://evil">` removed.
- Attacker `<meta http-equiv="Content-Security-Policy" content="connect-src *">` removed; tamtri CSP remains.
- External `<a href="https://example.com">` becomes `href="#"` with `data-tamtri-blocked-href` preserved.
- `collectBlockedHrefs` returns the blocked URL.

**Verify**: `pnpm --filter @tamtri/app run test` → all pass including new file

## Test plan

- New file: `packages/app/test/sandboxed-html.test.ts` — cases listed in Step 4.
- Structural pattern: `packages/app/test/conversation-surface.test.ts` (pure function tests, no daemon).
- Manual smoke (optional): open a conversation with an HTML artifact in right dock; preview renders; blocked external link appears in footer, not navigable.

## Done criteria

- [ ] `cargo test` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `pnpm run typecheck` exits 0
- [ ] `pnpm --filter @tamtri/app run test` exits 0; sandbox tests exist
- [ ] `rg 'allow-same-origin' packages/app/` returns no matches
- [ ] `rg 'contentWindow' packages/app/src/components/transcript/sandboxed-html.web.tsx` returns no matches
- [ ] No files outside in-scope list modified
- [ ] `plans/README.md` status row for 001 updated to DONE

## STOP conditions

- `sandboxed-html.web.tsx` no longer contains `sandbox="allow-same-origin"` at line ~70 (drifted or already fixed).
- Strict sandbox breaks all HTML preview rendering with no workaround — report with browser/Electron version.
- Hardening removes content required for hero demo `report.html` artifacts — report which tags are needed.
- Step verification fails twice after a reasonable fix attempt.

## Maintenance notes

- When MCP App panels ship, use a **separate** webview component with declared origins — do not reuse artifact sandbox settings.
- Reviewers should confirm iframe has **no** sandbox flags and sanitizer strips active content.
- Deferred: CSP `style-src 'unsafe-inline'` remains for rich reports; tightening is a separate effort.
