# Tamtri Visual QA Checklist

Run after visual polish changes on macOS 14+ in **dark and light** appearance.

## Build

```bash
./scripts/build-renderer.sh
cargo build -p tamtri-core
cd macos && swift build && swift test
```

## Chrome

- [ ] Sidebar header: toggle sidebar + search icons inline (Cursor-style), not in window toolbar
- [ ] New Conversation row at top of sidebar with ⌘N hint
- [ ] No sidebar toggle or new-conversation buttons in transcript toolbar (right panel toggle only)
- [ ] Sidebar nav: New chat, Search, Harness health as labeled rows (Codex-style)
- [ ] Chats section with single-line rows (title + compact time); no harness chips in list
- [ ] Settings-only footer; sidebar toggle bottom-right
- [ ] Sidebar shows relative times (no ISO timestamps)
- [ ] Sidebar groups Today / Yesterday / Earlier when 8+ conversations
- [ ] Sidebar rows show harness chip and running indicator for active conversation
- [ ] Sidebar footer exposes Settings and Harness Health
- [ ] Conversation header shows harness display name (e.g. Hermes), not `hermes-acp`
- [ ] Running indicator appears during active harness run (header + sidebar)
- [ ] Composer is a floating inset bar with harness/model chips and circular send
- [ ] Cancel replaces send in the same position while running
- [ ] Dropped files show as chips above composer
- [ ] Workspace layout matches Codex: left sidebar | center (header + transcript + composer) | optional right rail
- [ ] Center header: conversation title + actions; panel toggle in center header when rail closed
- [ ] Right rail open: center column narrows; toggle moves to rail header (no duplicate title bar)
- [ ] Closing files panel does not collapse the left sidebar

## Transcript

- [ ] Empty conversation shows hero empty state
- [ ] Transcript column is reading-width centered (~72ch)
- [ ] No User/Assistant debug labels on committed messages
- [ ] React renderer shows summary-first tool cards (not raw JSON by default)
- [ ] Thinking blocks collapsed by default
- [ ] Transcript scroll area fills space between header and composer
- [ ] Artifact cards are compact with Open preview action
- [ ] Native consent cards: Allow is primary (borderedProminent)
- [ ] Gateway events hide JSON behind "Show details"

## Files panel

- [ ] Workspace rail collapses; auto-opens when a new artifact completes
- [ ] Preview tab uses artifact typography for text deliverables
- [ ] No cramped 160pt list caps; preview gets ≥40% of files tab
- [ ] Frozen vs live badges visible on rows
- [ ] Selected row shows accent bar

## Accessibility

- [ ] Keyboard: sidebar → transcript → composer path works
- [ ] VoiceOver reads chip labels on header and file rows
- [ ] Reduce Motion: no essential information in animation-only cues

## Hero path

1. New conversation with Hermes
2. Drop CSV in composer
3. Ask for report
4. Verify artifact preview inline; workdir CSV stays secondary
