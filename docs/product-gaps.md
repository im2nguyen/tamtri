# Product And Spec Decisions

This file tracks product gaps that were promoted into V1 contracts during the M1/M2 reconciliation.

## Decided For V1

- **Onboarding:** first-run harness health screen with detected agents, missing installs, auth status, and copyable IT/admin setup checklist.
- **Artifact immutability:** transcript-rendered artifacts are always content-hashed snapshots under `attachments/`; `workdir/` remains messy, mutable, and local.
- **Consent UX:** permission cards name who is asking, show full diff or exact command, offer allow-once / allow-for-conversation / allow-for-folder-or-server, keep deny as prominent as allow, and never offer global forever-allow.
- **Tool settings:** gateway tools and agent-native tools are separated with scope labels.
- **Accessibility:** keyboard-first transcript, VoiceOver labels for all cards, Reduce Motion, WCAG AA contrast, Dynamic Type, and accessible fallback for webview content.
- **Search scope:** titles plus persisted `Text` and `Thinking` blocks only in V1.
- **Import integrity:** attachment hashes are verified; failed-integrity artifacts never render.
- **No telemetry:** learning comes through an opt-in diagnostics bundle the user can inspect.
- **Contribution mechanics:** CLA/DCO, `CONTRIBUTING`, Code of Conduct, and labeled issues before outside PRs.

## Still Worth Product Copy

Empty vault, malformed conversation, busy conversation, missing external folder bookmark, unsupported schema version, and unavailable harness need final user-facing copy when the Swift shell lands.
