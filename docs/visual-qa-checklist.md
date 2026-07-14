# Tamtri Visual QA Checklist

Run on macOS with the **Electron + Expo** shell (`@tamtri/app`).

## Build / run

```bash
cargo build -p tamtri-daemon
pnpm run app:web          # Terminal 1
pnpm run desktop:dev      # Terminal 2
```

Or packaged UI: `pnpm run desktop:build && pnpm --filter @tamtri/desktop run start`

## Project shell

- [ ] Sidebar contains only project rows with nested threads, search, create-project, and Settings
- [ ] Empty real projects remain visible and show `No threads yet`
- [ ] Unfiled is hidden when empty and appears for legacy/orphaned threads
- [ ] Unfiled cannot create threads, rename, delete, or attach shared roots
- [ ] Project disclosure, selection, rename, shared-root, delete, and new-thread actions work
- [ ] Resizing the sidebar persists after reload and clamps at both limits
- [ ] Main workspace is a raised content card at desktop widths
- [ ] Main and right-dock toolbars are 46 px high
- [ ] Frameless window drag regions do not cover interactive controls

### Desktop (at least 1100 px)

- [ ] Project sidebar and right dock can render inline together without collapsing the 640 px minimum main card
- [ ] Left and right resize handles clamp, persist after reload, and do not steal row/tab clicks
- [ ] Raised main card, workspace gutters, 46 px toolbars, and centered narrative column remain visually aligned

### 1024 px

- [ ] Project sidebar remains inline while the right dock opens as a dismissible overlay
- [ ] Main toolbar, project actions, composer controls, and `Scroll to latest message` do not clip
- [ ] Opening and closing the dock does not shift transcript scroll position or stale artifact selection

### Compact (less than 768 px)

- [ ] Project sidebar opens as a full-height dismissible overlay and selecting a thread closes it
- [ ] Project disclosure/actions remain reachable without hover
- [ ] Main card drops desktop-only gutters without horizontal overflow

## Transcript

- [ ] Empty conversation shows placeholder copy
- [ ] User messages as right-aligned pills; assistant as full-width column
- [ ] Thinking blocks in muted collapsible-style card
- [ ] Tool call + result cards
- [ ] Transcript and composer align to one centered ~820 px narrative column
- [ ] Scrolling up disables auto-follow; `Scroll to latest message` restores it
- [ ] Permission and elicitation cards remain keyboard-operable above the composer
- [ ] Reduce Motion removes nonessential disclosure, modal, and pane animation

## Settings

- [ ] `/settings` and unknown slugs redirect to General
- [ ] `/settings/agents` redirects to Providers
- [ ] Grouped settings navigation and search use compact sidebar rows
- [ ] Search result selection opens the correct section and scrolls to its row
- [ ] Compact layout exposes settings navigation and horizontal section chips
- [ ] Provider and usage pages distinguish unavailable, unauthenticated, and ready states
- [ ] Desktop settings use the dedicated grouped sidebar in the project-shell frame
- [ ] At 1024 px search results, section content, and row anchors fit without horizontal clipping
- [ ] Compact settings expose dismissible navigation plus horizontal section chips; selecting search results closes navigation and lands on the section

## Right dock and artifacts

- [ ] Dock is absent when the transcript has no artifacts, Apps, or tasks
- [ ] Artifact, App, and Task tabs appear only for available content
- [ ] Dock width persists and clamps; compact layout uses an overlay
- [ ] Verified text, CSV, image, and HTML artifacts render
- [ ] Failed-integrity artifacts never render their content
- [ ] HTML artifact scripts, remote images, external links, forms, and meta refresh cannot access the network
- [ ] Blocked artifact link attempts produce an audit receipt
- [ ] Artifact title, MIME type, size, and Show in Finder remain available outside HTML
- [ ] Tab roles, selected state, counts, close action, and artifact rows remain keyboard reachable

## Responsive and accessibility

- [ ] Project sidebar and right dock become dismissible overlays on narrow layouts
- [ ] Toolbar actions remain reachable without clipping at Dynamic Type sizes
- [ ] Every project/thread, toolbar, tab, consent, and dock action has a VoiceOver label
- [ ] Keyboard focus traverses transcript blocks and all card actions
- [ ] Light and dark themes meet WCAG AA contrast
- [ ] Receipt disclosures announce reasoning/tool/task/App labels, status, and detail actions

## Sessions import

- [ ] `/sessions` lists native Claude/Codex rows when available
- [ ] Import creates vault conversation and navigates to it

## Harness run

- [ ] Send message starts turn; transcript refreshes on completion
- [ ] Error state surfaces when daemon unreachable

## Live-run checks

- [ ] Streaming text does not trigger a maximum-update-depth loop
- [ ] Switching conversations closes stale dock selection
- [ ] Creating, moving, forking, exporting, and importing project conversations refreshes the sidebar
