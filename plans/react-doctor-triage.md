# React Doctor triage (2026-07-13)

Scan: `npx react-doctor@latest packages/app -y` from repo root (`/Users/dos/Desktop/tamtri`).

Baseline: score **49/100**, **104** issues (**14** errors, **90** warnings).

## Error rule (14×)

| Issue | Severity | Action | Commit |
| --- | --- | --- | --- |
| `no-impure-state-updater` (14 locations) | error | Mostly **false positive** (plain `setState(value)` / event handlers, not functional updaters). **fix** `onDrop` void-call pattern in composer + starter-screen | `fix(app): …` |

## Warnings (selected)

| Issue | Severity | Action | Commit |
| --- | --- | --- | --- |
| `public-env-secret-name` (2) | warning | **defer** — `EXPO_PUBLIC_DAEMON_TOKEN` is intentional local dev wiring in `app.config.js` / `connection-config.ts` | — |
| `no-array-index-as-key` (3) | warning | **fix** stable keys in settings + artifact preview | `fix(app): …` |
| `exhaustive-deps` (3) | warning | **defer** — needs per-hook review | — |
| `async-await-in-loop` (4) | warning | **defer** — sequential attach may be intentional | — |
| `no-effect-chain` (4) | warning | **defer** — harness/model picker flows | — |
| `rn-no-scrollview-mapped-list` (11) | warning | **defer** — short lists in sheets; FlashList migration later | — |
| Remaining maintainability/perf/a11y (~68) | warning | **defer** — batch in follow-up | — |

Full verbose output: `react-doctor-baseline.txt`.
