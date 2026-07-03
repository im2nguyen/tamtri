# Milestone 8: Product Completeness

Eighth build session. The app becomes usable by the target person, not just by the builder. The protocol and rendering substrate already exist; this milestone turns it into a calm, discoverable, accessible Mac app that can get a technical-adjacent user from first launch to the report demo without a terminal.

No new core thesis lands here. The work is onboarding, clarity, recovery, search, sharing, accessibility, diagnostics, and native affordances.

## Definition of done

- First-run harness health screen detects available ACP agents, auth/install status where possible, install-doc links, and a copyable IT/admin checklist.
- Search covers conversation titles plus `Text` and `Thinking` blocks, with the scope stated clearly in empty/no-result states.
- Share/export produces `.tamtri` bundles with `meta.json`, `messages.jsonl`, and `attachments/`; import verifies attachment hashes and blocks failed active content.
- Fork/share UX is polished: fork lineage is visible, "fork into" is easy to find, import creates a new conversation, and parent threads are never mutated.
- The six V1 error states are designed and implemented: empty vault, malformed conversation, busy conversation, missing external-folder bookmark, unsupported schema version, unavailable harness.
- Accessibility pass meets the V1 requirements: keyboard-first transcript, VoiceOver labels/values on every card type, Reduce Motion, AA contrast, Dynamic Type, and webview fallbacks.
- If the transcript/card UI is a React renderer island, it meets the same accessibility bar through WebKit semantics plus native fallback actions where needed.
- `issues()` from the vault is surfaced, including duplicate-folder badges and reveal-in-Finder actions.
- Diagnostics bundle assembles non-sensitive app/system/harness/log context for user-reviewed issue reports. Nothing leaves the machine automatically.
- Hotkeys, menu bar item, command palette, and cold-start performance budget are implemented.
- The app can run the hero flow end to end in a polished way on a fresh machine with a supported installed harness.

## Architecture note: product polish must not weaken the trust model

M8 is where it is tempting to add hidden convenience. Do not. The same rules still hold:

- No telemetry.
- No cloud.
- No opaque database as source of truth.
- No global forever-allow.
- No rendering tampered HTML.
- No secrets in diagnostics.
- No vault, gateway, credential, or permission ownership in the web renderer.

Every polish feature should make the local-first system more legible, not more magical.

## Task 1: First-run harness health screen

Build the screen that explains whether tamtri can actually run an agent.

Detect:

- known ACP agent binaries from the roster.
- basic version command when cheap and safe.
- auth status only when the harness exposes a non-invasive check.
- missing binary.
- installed but unauthenticated.
- installed and ready.
- incompatible or unknown version.

UI:

- first-run gate when no ready harness exists.
- settings entry to reopen anytime.
- install-doc links for each known harness.
- copyable IT/admin checklist: binaries needed, auth setup, keychain notes, and MCP server config location.
- manual refresh.

Rules:

- tamtri detects and guides. It does not bundle, install, or manage harnesses.
- Do not run prompts or network checks from the health screen without explicit user action.

Tests: missing harness, ready harness, auth unknown, auth failed, refresh, checklist copy, and unavailable-harness error state routes here.

## Task 2: Search

Implement V1 search exactly as specified:

- conversation titles.
- transcript `Text` blocks.
- transcript `Thinking` blocks.

Do not search:

- tool outputs.
- attachment contents.
- `workdir/`.
- audit logs.

Implementation may start with scanning vault files and add a rebuildable index only if needed for the performance budget. If an index is added, it is a cache and can be deleted/rebuilt without data loss.

UI:

- `Cmd-F` search in current context.
- global/conversation list search if already designed.
- highlighted matches in titles/transcript snippets.
- empty state states the scope plainly.

Tests: title match, text match, thinking match, no tool-output match, no attachment match, malformed conversation skipped with issue, and index rebuild if used.

## Task 3: Share, export, and import

Implement `.tamtri` bundle flow.

Export:

- zip `meta.json`, `messages.jsonl`, and `attachments/`.
- exclude `events.jsonl` and `workdir/` by default.
- hash-verify attachments before packaging.
- produce a deterministic enough layout for debugging.

Import:

- accept `.tamtri` bundle or conversation folder.
- verify every artifact hash and size.
- assign a new conversation id and clear `forked_from` for imported bundles.
- preserve transcript and attachments.
- if an artifact fails integrity, import the conversation but mark affected artifact blocks failed-integrity and never render active content.
- show import summary with warnings.

Fork UX:

- show fork lineage.
- reveal parent/child if present.
- preserve "fork into harness/model" from M4 with better copy and placement.

Tests: export excludes local files, import creates new id, hash mismatch marked failed, tampered HTML never reaches webview, missing attachment warning, fork lineage visible, and round-trip bundle reload.

## Task 4: Designed error states

Implement the six V1 error states with calm copy and one obvious recovery action.

1. Empty vault: create conversation or choose vault.
2. Malformed conversation: reveal in Finder, show issue detail, keep rest of app usable.
3. Busy conversation (`ConversationBusy`): show active run/cancel/open action.
4. Missing external-folder bookmark: re-pick folder or continue read-only.
5. Unsupported schema version: update app, reveal folder.
6. Unavailable harness: open harness health, fork into another harness if possible.

Rules:

- No raw Rust/Swift error dumps in primary UI.
- Details are available for diagnostics.
- Error states must be keyboard and VoiceOver accessible.

Tests: each state renders, recovery action works or routes correctly, details copy is available, and unrelated conversations remain browsable.

## Task 5: Accessibility pass

Treat accessibility as a release requirement.

Checklist:

- Full keyboard traversal of sidebar, transcript, composer, cards, webview fallbacks, settings, and dialogs.
- Focusable content blocks.
- Keyboard actions for expand diff, approve/deny consent, respond to elicitation, cancel task, reveal artifact.
- VoiceOver labels and values on tool cards, artifact cards, App panels, elicitation cards, task cards, permission cards, roots picker, and settings rows.
- React-rendered cards must expose equivalent semantics through WebKit accessibility, and Swift must provide native fallback actions for model-generated or inaccessible web content.
- Reduce Motion disables non-essential animation.
- WCAG AA contrast.
- Dynamic Type/responsive text without clipping.
- Webview fallback metadata available outside untrusted content.

Tests: accessibility identifiers/snapshots where useful, manual VoiceOver script, keyboard-only hero run, contrast audit, Reduce Motion smoke, and Dynamic Type layout check.

## Task 6: Vault issues and diagnostics bundle

Surface `issues()` and add user-reviewed diagnostics.

Vault issues:

- duplicate conversation id badge.
- unreadable/malformed folder warning.
- reveal in Finder.
- copy issue details.

Diagnostics bundle:

- app version, build, macOS version.
- renderer bundle version/build hash if a React renderer is shipped.
- harness roster and detected versions/status.
- gateway server config without secrets.
- recent non-sensitive events/log excerpts.
- recent crash/error summaries.
- redaction pass before writing.
- local file the user can inspect before attaching to a GitHub issue.

No automatic upload. No background analytics.

Tests: duplicate id surfaced, malformed folder surfaced, diagnostics excludes secret-like fields, user can reveal bundle, and logs are capped.

## Task 7: Native affordances

Add the Mac-app feeling:

- global launch hotkey, configurable.
- menu bar item.
- command palette (`Cmd-K`) for new conversation, fork, search, settings, harness health, reveal vault, diagnostics.
- standard shortcuts: `Cmd-N`, `Cmd-F`, send, cancel run.
- app settings for vault path, harness roster, gateway servers, credentials status, hotkey.
- cold-start performance budget with measurement.

Keep the first screen as the actual app, not a marketing page.

Tests: shortcuts route correctly, command palette actions work, menu bar opens app, hotkey configurable/disableable, settings persist, and cold-start measurement recorded.

## Task 8: Product QA and hero path

Run an end-to-end QA pass against the target user journey:

1. Fresh install.
2. Harness health identifies missing or ready harness.
3. Create a conversation.
4. Drag CSV.
5. Ask for report.
6. Approve permissions.
7. Inline `report.html` renders.
8. Search finds the conversation.
9. Export bundle.
10. Import bundle.
11. Fork into another harness/model.
12. Generate diagnostics bundle after an induced fixture error.

Record friction as issues before M9. M8 is done when this path feels boring in the best way.

## Enumerated tests

1. `harness_health_detects_missing_ready_and_unknown`.
2. `harness_health_checklist_copies`.
3. `search_matches_titles_text_and_thinking_only`.
4. `search_empty_state_names_scope`.
5. `export_bundle_excludes_events_and_workdir`.
6. `import_bundle_hash_verifies_attachments`.
7. `import_tampered_html_failed_integrity`.
8. `fork_lineage_visible`.
9. `empty_vault_state`.
10. `malformed_conversation_state`.
11. `busy_conversation_state`.
12. `missing_bookmark_state`.
13. `unsupported_schema_state`.
14. `unavailable_harness_state`.
15. `vault_duplicate_issue_badge`.
16. `diagnostics_bundle_redacts_secrets`.
17. `keyboard_only_hero_flow`.
18. `voiceover_card_labels`.
19. `command_palette_actions`.
20. `cold_start_budget_smoke`.

## Out of scope this milestone

Do not add cloud sync, accounts, telemetry, collaboration, team admin, marketplace discovery, or a public plugin system. Do not redesign the core protocol architecture. Do not add new MCP primitives beyond what M7 completed unless fixing a bug requires a small protocol patch.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, docs/milestone-8.md, docs/tamtri-decisions.md sections 1, 3, 8, 12, and 18, and docs/vault-format.md. Implement Milestone 8. Start with harness health, search scope, and import/export integrity, then stop and show me the user journey plus error-state map before doing the accessibility/native-affordance pass. This milestone is about making the local-first trust model feel clear and calm.
