# React Doctor triage (2026-07-13)

Scan: `pnpm dlx react-doctor@latest packages/app -y` from repo root (`/Users/dos/Desktop/tamtri`).

## Scores

| Pass | Score | Issues | Errors | Warnings |
| --- | --- | --- | --- | --- |
| Baseline (`c734b6b`) | 49/100 | 104 | 14 | 90 |
| After this pass (`fce331c`) | 51/100 | 85 | 14 | 71 |

Delta: **−19 issues**, **+2 score**. Errors unchanged (all 14 are triaged `no-impure-state-updater` false positives).

## Commits (this pass)

| Commit | Summary |
| --- | --- |
| `96070d7` | Nested sidebar buttons: outer `View` + inner `Pressable`; action `CompactIconButton`s as siblings |
| `aa0d9f5` | Hook deps, `Promise.all` workdir attaches, lazy connect-host init, `Set` project expansion, hoist `isNewlineKeyIntent` |
| `c799d01` | Stable list keys (settings warnings, CSV preview); bump left-sidebar/search badge text to `theme.fontSize.xs` |
| `fce331c` | Memoize projects `queryKey` to satisfy exhaustive-deps without per-render array recreation |

## Fixed in this pass

| Rule | Count | Notes |
| --- | --- | --- |
| Nested `<button>` hydration (manual) | 2 files | `project-row.tsx`, `thread-row.tsx` — no remaining `CompactIconButton` inside row `Pressable` |
| `no-array-index-as-key` | 3 | Stable keys in settings + artifact CSV preview |
| `exhaustive-deps` | 3 | `sidebar-resize-handle`, `use-projects` (`queryKey` memoized) |
| `async-await-in-loop` | 4 | `home-pane.tsx`, `use-workdir.ts` — parallel `Promise.all` |
| `rerender-lazy-state-init` | 2 | `connect-host-section.tsx` |
| `js-set-map-lookups` | 1 | `project-sidebar.tsx` expanded-project `Set` |
| `prefer-module-scope-pure-function` | 1 | `composer.tsx` `isNewlineKeyIntent` |
| `no-tiny-text` | 5+ | Sidebar rows, left-sidebar badges, search-result badge, project-sidebar labels (in commits above) |

## Still deferred

| Rule | Count | Action |
| --- | --- | --- |
| `no-impure-state-updater` | 14 | **skip** — false positives (plain `setState`, not functional updaters) |
| `public-env-secret-name` | 2 | **defer** — intentional `EXPO_PUBLIC_DAEMON_TOKEN` |
| `no-effect-chain` | 4 | **defer** — harness/model picker sheets need behavior review |
| `rn-no-scrollview-mapped-list` | 11 | **defer** — FlashList migration |
| `no-chain-state-updates` | 14 | **defer** — overlaps effect-chain refactors |
| Effect/parent-sync warnings | ~6 | **defer** — `run-recipe-sheet`, `use-resizable-sidebar-width`, `right-dock` |
| `rn-scrollview-dynamic-padding` | 6 | **defer** |
| `deslop/unused-file` + `unused-export` | 20 | **defer** — dead code audit |
| `no-giant-component` | 3 | **defer** |
| `no-tiny-text` | 2 | **defer** — `conversation-pane.tsx`, `right-dock.tsx` (uncommitted branch files; one-line fixes ready in working tree) |
| Other maintainability/perf | ~8 | **defer** — chained iterations, non-component exports |

## Verification

- `pnpm run typecheck` — pass
- `pnpm run test` — pass (40 tests)
- Sidebar grep: no `CompactIconButton` nested inside row-nav `Pressable` (`project-row`, `thread-row`, `left-sidebar` header actions are siblings)
