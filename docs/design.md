# tamtri design

tamtri uses a calm, information-dense workspace inspired by the interaction quality of Synara. The system is tamtri-owned and general-purpose. It serves analysts, marketers, operators, and other knowledge workers. It must not inherit Studio terminology, source-control chrome, or coding-only assumptions.

## Design spirit

- **Quiet structure.** Low-contrast surfaces establish hierarchy before borders do. One accent color communicates selection and action.
- **Compact, not cramped.** Rows are short and scan quickly, while controls keep accessible hit targets.
- **Narrative first.** The conversation is the primary object. Text, evidence, tool activity, and outputs read as one continuous story.
- **Progressive detail.** Common actions remain visible. Secondary controls appear through disclosure, hover on pointer devices, or always-visible native affordances.
- **Trust is visible.** Provenance, consent, integrity failures, and sandbox boundaries are part of the interface, not hidden implementation details.

## General-purpose boundary

Use task and knowledge-work language: projects, threads, files, outputs, agents, and providers. A project is an organizational container with optional shared roots, not a repository. Git state, terminals, branches, and code review are harness capabilities, not permanent shell navigation. The renderer displays daemon-owned state and emits intents; it never becomes an agent loop, vault owner, credential store, or MCP client.

## Shell grammar

The desktop shell has three regions:

1. A project-only left sidebar.
2. A raised main content card.
3. An optional right dock for artifacts, Apps, and tasks.

The sidebar contains projects and their threads. It does not mix top-level recent-thread or Studio navigation into the project tree. Real projects remain visible when empty. `Unfiled` appears only when it contains legacy or orphaned conversations.

The main content card sits above the workspace background with a restrained radius, border, and shadow. Its toolbar is 46 px high. Toolbar actions use compact labels or icons with accessible names. On compact layouts the sidebar and right dock become modal overlays.

## Visual tokens

- Geist is the interface face and Geist Mono is the code/data face. The base type scale is 12, 14, 16, and 18 px; user-configured interface and code sizes stay within their shipped bounds.
- Spacing follows a 4 px base scale: 4, 8, 12, 16, 20, 24, and 32 px. Density scales row, transcript gutter, composer, and settings spacing together; comfortable is the default.
- Radii progress from 4 to 12 px for controls and cards. The raised content card uses about 14 px; user bubbles about 13 px; circular controls use the full radius.
- The default left sidebar is 300 px and resizes from 208 to 480 px while preserving a 640 px main pane. The right dock defaults to 380 px, resizes from 280 to 600 px, stays inline from 1100 px, and otherwise overlays. Layout becomes compact below 768 px.
- Narrative content is capped at 820 px. Hairline borders are 1 px. Control, disclosure, and pane motion use approximately 150, 220, and 300 ms before Reduce Motion is applied.
- Colors are semantic, not component-owned: layered surfaces, foreground/muted foreground, border/accent border, green accent, destructive red, diff colors, popup surface, content seam, and shadow. Both light and dark themes use the same roles.

## Conversation grammar

The transcript and composer share one centered column. The preferred maximum width is approximately 820 px, with responsive side padding. Assistant output forms the narrative spine. User messages are visually distinct but do not dominate the page.

Tool calls, thinking, consent, Apps, tasks, and artifacts remain in transcript order. Summaries can appear in the right dock, but controls that affect a run stay in the transcript. The composer anchors the bottom of the narrative and keeps harness, model, attachment, and send state understandable without terminal vocabulary.

## Sidebar grammar

- Project rows use a disclosure affordance, project name, and compact contextual actions.
- Thread rows nest under projects and use the same row rhythm as settings navigation.
- Empty projects show a quiet `No threads yet` row.
- `Unfiled` is immutable and cannot create threads or own shared roots.
- Search temporarily replaces the tree and states its transcript-only scope.

## Settings grammar

Settings reuse the shell rather than opening a disconnected utility page. A dedicated left navigation groups app, agents, connection, data, and advanced sections. Search returns setting rows and moves to stable anchors. Invalid slugs redirect to General; the legacy Agents route redirects to Providers.

Content uses a centered reading column, clear section titles, compact rows, and inline explanations. Compact layouts retain section chips in content when the sidebar is hidden. Provider screens distinguish agent-native capabilities from tamtri gateway capabilities.

## Motion and accessibility

All actions have keyboard and VoiceOver semantics. Project disclosures, transcript receipts, tabs, consent decisions, and dock controls expose role, label, value/status, and selected/expanded state where applicable. Selection uses more than color alone. Text and controls meet WCAG AA contrast. Dynamic Type and responsive widths must not clip primary actions.

Honor Reduce Motion. Web motion tokens become zero-duration under `prefers-reduced-motion`; native transitions should avoid nonessential movement when the platform setting is enabled. Pointer-only reveal is never the only route to an action: controls remain visible on native and compact layouts. Model-generated content always has an accessible title, type, and safe reveal action outside its sandbox.

## Security surfaces

Artifact HTML is verified before rendering, stripped of external URL-bearing attributes, and served in a scriptless iframe with a deny-by-default CSP. Artifact previews have no network access. MCP Apps use their separate declared-origin and consent policy. Neither surface can read the vault, credentials, or daemon directly.

Permission and elicitation cards name who is asking, show the exact action or readable argument summary, keep deny as prominent as allow, and route every response through daemon consent and audit. Integrity failures remain visible but never pass bytes to an active renderer.
