# React transcript implementation plan

Branch: `experiment/react-transcript`

Goal: one React renderer for the entire transcript scroll area (committed + live), using shadcn for chrome. Swift/core keeps vault, harness, gateway, and consent decisions.

## Golden rules (non-negotiable)

| Layer | Owns |
|-------|------|
| **Rust core** | Conversation model, vault, harness runs, MCP gateway, event reduction, audit log |
| **Swift shell** | App lifecycle, window chrome, sidebar, composer, settings, keychain, bookmarks, permission *decisions*, fork/import/export, files rail |
| **React renderer** | Presentation only: layout, typography, markdown, disclosures, hover chrome, card shells |
| **Sandboxed WKWebView hosts (separate)** | Untrusted HTML: harness artifacts, MCP App templates |

React must **never** read the vault, spawn harnesses, hold credentials, or approve permissions on its own. Every user action is an **intent** dispatched to Swift; Swift uses the same paths as today (`respondPermission`, `send`, fork sheet, etc.).

**shadcn for chrome, not for trust.** Message bubbles, tool cards, permission card *frames*, hover actions, collapsibles — yes. Rendering `report.html` or MCP App HTML inside the main React tree — no.

---

## Current state

```
TranscriptView (Swift)
├── TranscriptRendererSection
│   ├── React batch (WKWebView) — text/thinking/tools only
│   └── Native MessageRow — permissions, artifacts, apps, fallback
└── LiveTranscriptSection (Swift only) — streaming events
```

Problems this causes:

- Duplicate presentation logic (Swift + React)
- Recent UX polish landed on native path; React path lags
- Live vs committed use different component models
- Height/sync hacks for embedded WKWebView batches

---

## Target architecture

```
TranscriptView (Swift — thin host)
└── TranscriptWebHost (single WKWebView, full scroll height)
    └── React root
        ├── CommittedMessageList
        ├── LiveTurnSection (optional, same components)
        └── Scroll anchor / empty state

Swift ──view model JSON──► React
React ──intent JSON──────► Swift ──► AppStore / core
```

Native Swift **keeps**:

- `SandboxedHTMLView` for artifact preview (files rail) and inline artifact *frames* that only show metadata + "Open preview" (intent → Swift opens rail/webview host)
- `AppPanelView` sandbox for MCP Apps (or a dedicated second webview slot Swift mounts when React emits `openAppPanel`)
- Composer, sidebar, modals, fork picker, settings

Native Swift ** deletes** (after parity):

- `MessageRow`, most of `TranscriptCards.swift` message rendering
- `LiveTranscriptSection` as a separate Swift tree
- `TranscriptRendererSection` batch/split logic

Keep a **minimal native fallback** only when `renderer/dist` is missing (dev without `npm run build`).

---

## Bridge contract

### Swift → React (`TranscriptViewModel`)

Push on every relevant `AppStore` change (messages, live events, run state, theme).

```typescript
type TranscriptViewModel = {
  schema_version: 1;
  conversation_id: string;
  color_scheme: "light" | "dark";
  run: {
    active: boolean;
    harness_id?: string;
    model_id?: string;
  };
  messages: TranscriptMessage[];
  live?: LiveSegment; // null when idle
};

type TranscriptMessage = {
  id: string;
  role: "user" | "assistant";
  created_at?: string; // ISO8601
  harness_id?: string;
  is_latest_user: boolean;
  is_latest_assistant: boolean;
  blocks: ContentBlock[];
};

type LiveSegment = {
  activity: ActivityItem[];
  text_stream?: string;
  permission?: PermissionRequest | null;
};
```

`ContentBlock` mirrors persisted transcript blocks. Rich types carry **display fields only**, not file bytes or secrets:

| Block type | React renders | Trust boundary |
|------------|---------------|----------------|
| `text` | Markdown + inline code | Safe (user/model text) |
| `thinking` | Disclosure "Thought for Ns" | Safe |
| `tool_call` / `tool_result` | Tool card + monospace output | Safe (already redacted in transcript) |
| `artifact` | **Card shell**: title, mime, size, integrity badge | No HTML inline; `openArtifact` intent |
| `elicitation_request` | Form shell or URL consent CTA | Submit → intent; Swift validates |
| `elicitation_response` | Compact receipt | Safe |
| `app_resource` | **Card shell**: server, template ref | `openAppPanel` intent → Swift mounts sandbox |
| `task_ref` | Task status card | Poll via Swift; React shows last snapshot |
| permission (in tool_result) | Consent card with Allow/Deny | `respondPermission` intent |

### React → Swift (`TranscriptIntent`)

Register handler: `window.webkit.messageHandlers.tamtriIntent.postMessage(intent)`.

```typescript
type TranscriptIntent =
  | { type: "copy_text"; text: string }
  | { type: "edit_message"; message_id: string }
  | { type: "retry_user"; message_id: string }
  | { type: "retry_assistant"; message_id: string }
  | { type: "fork_conversation" }
  | { type: "respond_permission"; request_id: string; option_id: string }
  | { type: "submit_elicitation"; request_id: string; action: string; data?: unknown }
  | { type: "open_artifact"; message_id: string; path: string; sha256: string; size: number }
  | { type: "open_app_panel"; message_id: string; block_index: number }
  | { type: "open_workdir_file"; relative_path: string }
  | { type: "report_height"; height: number }
  | { type: "focus_composer" };
```

Swift maps each intent to existing `AppStore` methods. No new business logic in the bridge — only routing + validation (conversation id match, run active for permission, etc.).

### Theming

- Swift sends `color_scheme` and optionally CSS variable overrides synced from `TamtriTheme` / `design-tokens.json`.
- shadcn uses CSS variables (`--background`, `--foreground`, …) aligned with existing `renderer/src/tokens.css`.
- Run `scripts/sync-design-tokens.sh` when native tokens change.

---

## Renderer stack (shadcn)

1. Add Tailwind + shadcn/ui to `renderer/` (Vite-compatible).
2. Pin components we need first (don't install the whole catalog):
   - `Button`, `Tooltip`, `Collapsible`, `Separator`, `Badge`, `ScrollArea` (if needed inside host)
3. Keep `react-markdown` for text blocks; add `remark-gfm` if tables needed.
4. Structure:

```
renderer/src/
  bridge/           intent.ts, viewModel.ts, useBridge()
  transcript/
    TranscriptRoot.tsx
    MessageRow.tsx
    MessageActionBar.tsx
    blocks/         Text, Thinking, Tool, ArtifactShell, Permission, …
    live/           LiveTurn.tsx
  components/ui/    shadcn
  tokens.css
```

---

## Migration phases

### Phase 0 — Scaffold (this branch)

- [ ] Add shadcn + Tailwind; verify dark mode in WKWebView
- [ ] Define `TranscriptViewModel` / `TranscriptIntent` TypeScript types + Swift encoders
- [ ] Add `tamtriIntent` WKScriptMessageHandler alongside `tamtriHeight`
- [ ] Single full-height `TranscriptWebHost` behind feature flag `UserPreferences.useReactTranscript` (default off until Phase 3)

### Phase 1 — Committed messages parity

- [ ] Message bubbles (user grey pill, assistant plain column)
- [ ] Markdown + inline code pills (match current tokens)
- [ ] Message action bar (hover + persistent latest assistant): copy, retry, edit, fork, date
- [ ] Activity clusters: thought disclosure with duration, tool cards, cluster summary row
- [ ] Permission consent cards → `respondPermission` intent
- [ ] Stale permission → muted receipt

**Exit:** Hero conversation replay renders correctly with no native `MessageRow` for standard messages.

### Phase 2 — Live streaming in React

- [ ] Map `AppStore.liveEvents` → `LiveSegment` (reuse `LiveActivityGrouping` logic in Rust or Swift encoder — prefer **Swift encoder** mirroring existing grouping to avoid core churn)
- [ ] Merge consecutive text/thought deltas (same rules as today)
- [ ] Live permission card while run active
- [ ] Remove `LiveTranscriptSection` Swift tree when flag on

**Exit:** Full harness run visible entirely in React; no flash on commit (Swift sends committed message + clears live in one payload).

### Phase 3 — Rich block shells (still no untrusted HTML in tree)

- [ ] Artifact card shell → `openArtifact` / files rail
- [ ] Elicitation form (structured) → intent; URL mode → Swift opens trusted handoff
- [ ] Task ref card (status from block snapshot)
- [ ] App resource shell → Swift mounts existing `AppPanelView` overlay or side slot

**Exit:** No native transcript rendering except sandbox hosts.

### Phase 4 — Cleanup

- [ ] Delete dead Swift: `TranscriptCards` message paths, `TranscriptRendererSection` splitting, duplicate presentation builders unused by shell
- [ ] Flip feature flag default **on**; keep native fallback one release
- [ ] Accessibility pass: keyboard focus into webview, roving tabindex on messages, ARIA on disclosures/buttons
- [ ] Update `docs/visual-qa-checklist.md`

---

## Accessibility (V1 requirement)

React must implement:

- Focusable message rows and action buttons
- `aria-expanded` on disclosures
- Visible focus rings (shadcn defaults + token tweak)
- Live region for streaming text (`aria-live="polite"`)
- Swift provides accessibility fallback metadata on artifact/app shells for VoiceOver outside the webview when content is untrusted

---

## Testing

| Layer | What |
|-------|------|
| **TypeScript** | View model segmenting, intent builders, markdown normalization (existing + new) |
| **Swift** | Bridge encode/decode round-trip, intent routing → AppStore mocks |
| **Integration** | RendererPolicyTests → full payload snapshots; relaunch with committed + live |
| **Manual** | `docs/visual-qa-checklist.md` + hero CSV → report demo |

---

## Out of scope for this experiment

- Electron or replacing Swift shell
- React owning vault or harness lifecycle
- Inline rendering of artifact HTML or MCP App HTML in the main transcript tree
- Cross-platform shells (GTK/WinUI)

---

## Success criteria

1. One visual system (shadcn) for all transcript chrome
2. Zero duplicate Swift/React presentation logic for messages
3. Permission and tool flows unchanged at the core/gateway layer
4. Hero demo works: user message → thought/tools → permission → artifact card → files rail, with React rendering the transcript
5. `swift test` + renderer build green; feature flag allows rollback
