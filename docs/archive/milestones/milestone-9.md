# Milestone 9: Ship It

Ninth build session. Release engineering becomes real. The app is signed, notarized, packaged, updateable, tested in CI, documented for users and contributors, and ready for a `v0.1.0` announcement around the report-from-data demo.

This milestone should not add product surface area unless a release blocker demands it. The job is to make the thing people already tested in M8 safe to download, understandable, maintainable, and supportable.

## Definition of done

- macOS app is signed, notarized, packaged as a DMG, and installable on a clean supported machine.
- Auto-update mechanism is configured and tested against a staged update feed.
- Homebrew cask is prepared or documented for submission.
- CI runs Rust tests, clippy, Swift build, fixture tests, and UI smoke on every PR. If a React/TypeScript renderer is shipped, CI also runs renderer typecheck, tests, and production bundle build.
- Release pipeline builds from a tag, produces artifacts, signs/notarizes them, and emits checksums.
- Contribution mechanics are live before the announcement: CLA or equivalent contributor agreement, DCO, CONTRIBUTING.md, code of conduct, issue labels, and good-first-issue seeds.
- README tells the product story, shows the hero demo clip, states requirements, and links to docs.
- Docs site or docs index is seeded from `/docs` and has an obvious "start here" path.
- Security/privacy review checklist is complete: webview sandbox, keychain, diagnostics redaction, bundle import integrity, and no telemetry.
- `v0.1.0` release notes are written, honest about limitations, and include the supported harness/server matrix.
- Final manual QA passes on a clean machine from download to hero demo.

## Architecture note: release work is part of the product

The trust promise is local-first, open-source, and user-owned. Release mechanics must reinforce that:

- signed and notarized binaries so users are not trained to bypass macOS safety.
- checksums and source tags so builds are auditable.
- no telemetry or silent network calls.
- clear license and contribution terms before outside PRs.
- documented limitations instead of surprise failure states.

## Task 1: Signing, notarization, and packaging

Set up macOS distribution.

Requirements:

- Developer ID signing for the app and helpers.
- Hardened runtime with only required entitlements.
- Notarization in the release pipeline.
- Stapled notarization ticket.
- DMG with app bundle and a clear Applications shortcut.
- Version/build metadata visible in About and diagnostics.
- Local verification script for signature, notarization, entitlements, and launch.

Tests/checks:

- install on a clean supported macOS user account.
- Gatekeeper accepts the app without bypass.
- app launches with an empty vault.
- helper binaries are signed.
- entitlements are minimal and reviewed.

## Task 2: Auto-updates

Add Sparkle or an equivalent native updater.

Requirements:

- update feed generation.
- EdDSA/signature keys stored outside the repo.
- staged test feed for pre-release validation.
- user-visible update UI.
- no forced background download unless the chosen updater and settings make that explicit.
- rollback story documented, even if manual for v0.1.0.

Tests/checks:

- install v0.1.0-rc1.
- update to v0.1.0-rc2 from staged feed.
- failed update leaves the old app usable.
- diagnostics include current version/build/update channel.

## Task 3: CI and release pipeline

Build the automation that keeps the repo shippable.

PR CI:

- `cargo test`.
- `cargo clippy`.
- Rust formatting check if the repo standardizes on it.
- Swift build.
- Swift/unit tests where present.
- renderer typecheck/test/build if `/renderer` exists.
- fixture tests for ACP/MCP stdio and HTTP.
- UI smoke for launch and basic conversation render where feasible.

Release CI:

- tag-driven build.
- produce app archive and DMG.
- include the renderer production bundle in signed app resources if `/renderer` exists.
- sign and notarize.
- generate checksums.
- generate/update appcast if using Sparkle.
- attach artifacts to GitHub release draft.

Rules:

- Secrets live in CI secret storage, not the repo.
- Release pipeline must not require a developer's laptop for normal releases.
- Nightly or manual matrix can cover slower real-agent checks; PR CI stays hermetic.

Tests/checks: intentionally failing PR proves CI blocks merge, release dry run on an RC tag, artifact checksum verification, and appcast validation.

## Task 4: Contribution mechanics

Prepare the project for outside contributors.

Must exist before announcement:

- AGPL license clearly stated.
- CLA or copyright assignment flow that preserves the future MIT relicense option.
- DCO signoff policy if used alongside CLA.
- CONTRIBUTING.md with setup, test commands, doc style, and PR expectations.
- Code of conduct.
- Security policy and responsible disclosure contact/path.
- Issue labels: area/core, area/macos, area/mcp, area/harness, area/docs, good-first-issue, help-wanted, needs-repro, security.
- Good-first issues mapped to docs/tests/small UI polish.

Tests/checks: PR template present, issue templates present, CLA/DCO check wired if chosen, and a new contributor can follow CONTRIBUTING.md on a clean checkout.

## Task 5: README, docs index, and demo assets

Turn the internal brief into public-facing docs.

README must include:

- one-paragraph product framing.
- hero demo clip or GIF.
- install/download instructions.
- supported macOS version.
- supported harness roster and what "installed separately" means.
- quickstart: create conversation, attach/drop CSV, ask for report.
- security/local-first promises.
- current limitations.
- build from source.
- license.

Docs:

- architecture overview linked from README.
- vault format.
- MCP gateway overview.
- harness adapter overview.
- troubleshooting/harness health.
- privacy/security page.
- release notes.

Rules:

- Do not overpromise model quality or harness compatibility.
- Explain "tamtri owns the conversation; harness/model fixed per thread; fork to change either" plainly.
- Show the report demo, not a generic chat screenshot.

Tests/checks: README links valid, docs index links valid, demo asset loads, quickstart verified on a clean machine.

## Task 6: Security and privacy release review

Run an explicit release gate.

Checklist:

- Artifact webview: no network, no host bridge.
- Trusted React renderer island, if present: narrow bridge, reviewed CSP, and no vault/gateway/credential/permission ownership.
- App webview: declared origins only, consent-gated host bridge.
- Elicitation: secret-looking fields blocked in form mode.
- URL handoff/OAuth: exact host consent, tokens in keychain, no token logs.
- Gateway credentials: references in vault, values only in keychain/memory.
- Import: tampered active content never renders.
- Diagnostics: redaction pass, user-reviewed, no automatic upload.
- Permissions: no global forever-allow.
- Storage: transcript remains legible, audit log local, secrets absent.
- Network: no telemetry, no unexpected calls at launch.

Produce a short `docs/security-review-v0.1.0.md` or release checklist entry with pass/fail notes and known residual risks.

Tests/checks: run targeted security tests from M5-M8, inspect entitlements, inspect network behavior during launch/hero flow, and verify diagnostics bundle manually.

## Task 7: Supported matrix and release candidate QA

Create the v0.1.0 compatibility matrix.

Matrix:

- macOS versions supported.
- Apple Silicon and Intel if supported.
- harnesses: Claude Code ACP, Gemini, Goose, Hermes, or whichever roster is actually ready.
- downstream MCP servers: stdio fixture, HTTP fixture, one real local server, one real remote server if available.
- key flows: first launch, create conversation, permissions, gateway tool, artifact render, elicitation, App, Task, Roots, export/import, diagnostics.

Release candidate loop:

1. Tag RC.
2. Build signed/notarized DMG.
3. Install on clean machine.
4. Run full matrix.
5. File blockers.
6. Fix only blockers.
7. Repeat until clean.

Do not keep adding features during RC. Bugs only.

## Task 8: v0.1.0 release

Final release work:

- create release branch or tag according to repo policy.
- update version numbers.
- update changelog/release notes.
- generate signed/notarized artifacts.
- publish GitHub release.
- publish update feed.
- publish Homebrew cask PR or instructions.
- announce with the report demo.
- pin known limitations and next milestones.

Release notes should name what works and what does not:

- model/harnesses are user-installed.
- sampling is declined by design.
- no cloud/accounts/telemetry.
- V1 is macOS only.
- agent-native MCP servers may exist outside tamtri's gateway and are labeled separately.

## Enumerated checks

1. `release_build_from_clean_checkout`.
2. `cargo_test_ci`.
3. `cargo_clippy_ci`.
4. `swift_build_ci`.
5. `renderer_build_ci` if `/renderer` exists.
6. `fixture_smoke_ci`.
7. `ui_launch_smoke_ci`.
8. `dmg_signature_verified`.
9. `notarization_verified`.
10. `updater_staged_feed_success`.
11. `updater_failed_update_keeps_old_app`.
12. `diagnostics_redaction_release_check`.
13. `webview_artifact_no_network_release_check`.
14. `trusted_renderer_bridge_release_check` if `/renderer` exists.
15. `oauth_token_not_in_logs_release_check`.
16. `tampered_bundle_import_release_check`.
17. `clean_machine_hero_flow`.
18. `readme_quickstart_verified`.
19. `contributing_clean_checkout_verified`.
20. `cla_or_dco_check_enabled`.
21. `release_artifacts_checksummed`.
22. `v0_1_0_release_notes_complete`.

## Out of scope this milestone

Do not add cloud sync, accounts, telemetry, team collaboration, paid features, marketplace discovery, Windows/Linux shells, or a public harness plugin SDK. Do not broaden the supported matrix after RC unless a harness is already working and documented. Do not redesign the product during ship week.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-9.md, docs/tamtri-decisions.md sections 3, 11, 12, and 18, CONTRIBUTING.md, and CODE_OF_CONDUCT.md. Implement Milestone 9. Start by creating the release checklist, CI matrix, and signing/notarization plan, then stop and show me the release pipeline before wiring updater and public docs. No new product features unless they unblock v0.1.0.
