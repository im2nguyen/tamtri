# Tamtri Visual QA Checklist

Run on macOS with the **Electron + Expo** shell (`@tamtri/app`).

## Build / run

```bash
cargo build -p tamtri-daemon
npm run app:web          # Terminal 1
npm run desktop:dev      # Terminal 2
```

Or packaged UI: `npm run desktop:build && npm run start --workspace @tamtri/desktop`

## Chrome (Paseo-inspired)

- [ ] Left sidebar: search, new conversation, conversation rows with status dot
- [ ] Sidebar footer: Import sessions link
- [ ] Home empty state when no conversation selected
- [ ] Conversation header: title + harness/model chips
- [ ] Composer: floating inset bar, circular send, VaultLocal chip
- [ ] Frameless window drag region (desktop)
- [ ] Dark theme: sidebar `#141716`, workspace `#181B1A`, green accent

## Transcript

- [ ] Empty conversation shows placeholder copy
- [ ] User messages as right-aligned pills; assistant as full-width column
- [ ] Thinking blocks in muted collapsible-style card
- [ ] Tool call + result cards
- [ ] Transcript column centered (~820px max width)

## Sessions import

- [ ] `/sessions` lists native Claude/Codex rows when available
- [ ] Import creates vault conversation and navigates to it

## Harness run

- [ ] Send message starts turn; transcript refreshes on completion
- [ ] Error state surfaces when daemon unreachable

## Not in this pass

- Artifact HTML webview panels, MCP Apps, consent cards, live token streaming, settings
