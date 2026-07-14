# Product gaps

Tracked UX and polish gaps. Implementation status lives in [README.md](./README.md#product-status); this file is for copy and product decisions still open.

## Decided for V1

- **Onboarding:** First-run flow (welcome → pre-recorded example → gate → explicit Run sample → own-file payoff) with adapter readiness diagnostics — **implemented** (see [quickstart.md](./quickstart.md), [observation-sessions.md](./observation-sessions.md))
- **Artifact immutability:** transcript artifacts are content-hashed snapshots under `attachments/`; `workdir/` stays mutable and local
- **Consent UX:** permission cards name who is asking, show full diff or exact command, scope choices, no global forever-allow
- **Tool settings:** gateway tools vs agent-native tools separated with scope labels
- **Accessibility:** keyboard-first transcript, VoiceOver, Reduce Motion, WCAG AA — **partial pass**
- **Search scope:** titles plus `Text` and `Thinking` blocks only
- **Import integrity:** attachment hash verification; failed integrity never renders
- **No telemetry:** opt-in diagnostics bundle only

## Copy and UX still open

| State | Needs |
|-------|--------|
| Empty vault / first launch | Plain-language path to Agents & providers (onboarding flow planned) |
| Malformed conversation | Calm recovery copy + reveal in Finder |
| Busy conversation | Wait or cancel guidance |
| Missing external-folder bookmark | Re-pick folder flow |
| Unsupported schema version | Update app message |
| Unavailable harness | Link to Agents & providers, not dev commands |
| Daemon connection failure | "Restart tamtri" for packaged app; dev hints only in `__DEV__` |

## Packaging

- Signed/notarized Mac DMG and auto-update (today: `pnpm run dev:desktop`)
- Relay remote access for mobile without LAN

## Contribution mechanics

CLA/DCO, `CONTRIBUTING`, Code of Conduct, and labeled issues before outside PRs.
