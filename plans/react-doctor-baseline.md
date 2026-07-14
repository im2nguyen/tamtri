React Doctor v0.7.7

  

  ⚠ Security: Secret-like public env variable ×2
    Client code references a public env variable whose name
    looks like a secret or privileged credential.
    → Public env prefixes are inlined into browser bundles.
    Rename public values to non-secret names, and keep tokens,
    passwords, private keys, and service-role credentials
    server-only.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/public-env-secret-name

    app.config.js:7

    src/runtime/connection-config.ts:54

  ⚠ Bugs: Array index used as a key ×3
    Your users can see & submit the wrong data when this list
    reorders or filters, so use a stable id like
    `key={item.id}`, not the array index "index".
    → Use a stable id from the item, like `key={item.id}` or
    `key={item.slug}`. Index keys break when the list reorders
    or filters.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-array-index-as-key

    src/components/artifact/artifact-preview-panel.tsx:47

    src/screens/settings/advanced-section.tsx:62

    src/screens/settings/import-bundle-section.tsx:77

  ⚠ Bugs: Missing effect dependencies ×3
    `useMemo` can run with a stale `theme.colors.border` & show
    your users old data.
    → Don't blindly add missing dependencies. Read the hook
    callback first.
    
    Bad:
    useEffect(() => {
      setCount(count + 1);
    }, [count]);
    
    Better:
    useEffect(() => {
      setCount((currentCount) => currentCount + 1);
    }, []);
    
    If the missing value is recreated every render, move it
    inside the hook or stabilize it before adding it to deps.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/exhaustive-deps

    src/components/layout/sidebar-resize-handle.tsx:56

    src/components/layout/sidebar-resize-handle.tsx:68

    src/hooks/use-projects.ts:26

  ⚠ Performance: await inside a loop ×4
    This makes the for…of loop slow because each await runs one
    after another, so collect the independent calls & run them
    together with `await Promise.all(items.map(...))`
    → Collect the items, then use `await
    Promise.all(items.map(...))` so independent work runs at
    the same time
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/async-await-in-loop

    src/components/layout/home-pane.tsx:187

    src/components/layout/home-pane.tsx:195

    src/hooks/use-workdir.ts:43

    src/hooks/use-workdir.ts:62

  ⚠ Bugs: Effects chained together ×4
    Your screen redraws several times from a single action
    because one useEffect changes "selectedHarnessId", which
    sets off this one.
    → Compute as much as possible during render (e.g. `const
    isGameOver = round > 5`) and write all related state inside
    the event handler that originally fires the chain. Each
    effect link adds an extra render and makes the code rigid
    as requirements evolve
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-effect-chain

    src/components/layout/home-pane.tsx:71

    src/components/layout/home-pane.tsx:92

    src/components/sidebar/fork-conversation-sheet.tsx:77

    src/components/sidebar/new-conversation-sheet.tsx:75

  ⚠ Bugs: Non-virtualized mapped list in ScrollView ×11
    Your users get slow scrolling when <ScrollView> with
    items.map(...) builds every row at once.
    → `<ScrollView>{items.map(...)}</ScrollView>` builds every
    row at once, which slows scrolling. Use FlashList,
    LegendList, or FlatList instead.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/rn-no-scrollview-mapped-list

    src/components/artifact/right-dock.tsx:74

    src/components/artifact/right-dock.tsx:136

    src/components/composer/harness-picker-sheet.tsx:122

    src/components/composer/model-picker-sheet.tsx:146

    src/components/orchestration/run-recipe-sheet.tsx:194

    src/components/sidebar/fork-conversation-sheet.tsx:145

    src/components/sidebar/fork-conversation-sheet.tsx:167

    src/components/sidebar/new-conversation-sheet.tsx:140

    src/components/sidebar/new-conversation-sheet.tsx:162

    src/screens/settings-screen.tsx:110

    src/screens/settings/import-sessions-section.tsx:87

  ⚠ Bugs: Parent kept in sync with a callback effect
    Your parent re-renders on every local state change because
    this useEffect calls the prop "onComplete" just to stay in
    sync.
    → Move the shared state into a Provider so both sides read
    the same value. Then you don't need a useEffect to keep
    them in sync.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-prop-callback-in-effect

    src/components/orchestration/run-recipe-sheet.tsx:103

  ⚠ Bugs: Live state pushed to parent via effect
    Pushing state up to a parent from a useEffect costs your
    users an extra render.
    → Move the state up to the parent (or return it from the
    hook), instead of handing it back up through a prop
    callback in a useEffect. See
    https://react.dev/learn/you-might-not-need-an-effect#notifying-parent-components-about-state-changes
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-pass-live-state-to-parent

    src/hooks/use-resizable-sidebar-width.ts:33

  ⚠ Maintainability: deslop/unused-file ×6
    Unused file is not reachable from any entry point, so it
    adds maintenance surface without shipping any code.
    → Delete the file if it is truly unreachable, or import it
    from an entry point.

    src/components/artifact/artifact-sidebar.tsx

    src/components/health/provider-catalog.tsx

    src/components/sidebar/conversation-row.tsx

    src/components/sidebar/new-conversation-sheet.tsx

    src/data/acp-provider-catalog.ts

    src/styles/theme.ts

  ⚠ Bugs: All state reset on prop change
    Your users briefly see stale state when a prop changes
    because this useEffect clears all state.
    → Pass the prop as `key` so React resets the component for
    you when the prop changes, instead of clearing every state
    value by hand in a useEffect. See
    https://react.dev/learn/you-might-not-need-an-effect#resetting-all-state-when-a-prop-changes
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-reset-all-state-on-prop-change

    src/components/artifact/right-dock.tsx:172

  ⚠ Performance: Array lookup inside a loop
    This scales poorly because `array.includes()` inside a loop
    scans the whole list every time. Use a Set for
    constant-time lookups.
    → Use a `Set` or `Map` when you check for the same items
    over and over. `Array.includes`/`find` scans the whole list
    each time
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/js-set-map-lookups

    src/components/sidebar/project-sidebar.tsx:62

  ⚠ Accessibility: Text is too small ×5
    Your users strain to read 10px text, so use at least 12px
    for body text, & 16px is best.
    → Use at least 12px for body text, and 16px is best. Small
    text is hard to read, especially on phones.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-tiny-text

    src/components/artifact/right-dock.tsx:56

    src/components/layout/conversation-pane.tsx:451

    src/components/sidebar/left-sidebar.tsx:133

    src/components/sidebar/project-sidebar.tsx:112

    src/components/sidebar/thread-row.tsx:69

  ⚠ Bugs: State updates chained through effects ×14
    Chaining state updates triggers an extra render each step.
    → Set all the related state together in the event handler
    that starts it, instead of having one useEffect react to a
    state change and set more state. See
    https://react.dev/learn/you-might-not-need-an-effect#chains-of-computations
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-chain-state-updates

    src/components/health/add-agent-sheet.tsx:95

    src/components/layout/home-pane.tsx:73-74

    src/components/orchestration/run-recipe-sheet.tsx:88-91

    src/components/orchestration/run-recipe-sheet.tsx:97

    src/components/sidebar/fork-conversation-sheet.tsx:68-69

    src/components/sidebar/fork-conversation-sheet.tsx:79

    src/components/sidebar/new-conversation-sheet.tsx:66-67

    src/components/sidebar/new-conversation-sheet.tsx:77

  ⚠ Performance: State initializer runs on every render ×2
    useState(defaultDirectConfig()) re-runs
    defaultDirectConfig() on every render & throws the result
    away.
    → Wrap expensive initial state in an arrow function so the
    initializer does not rerun and get thrown away on every
    render.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/rerender-lazy-state-init

    src/screens/settings/connect-host-section.tsx:21-22

  ⚠ Maintainability: deslop/unused-export ×14
    Unused export: `ThinkingIndicator` is exported but no
    module imports it, so it expands the public surface and can
    mislead callers about supported API.
    → Drop the `export` keyword (or remove the declaration) if
    no other module uses this symbol.

    src/components/transcript/message-list.tsx:442

    src/components/transcript/sandboxed-html.tsx:14

    src/components/transcript/sandboxed-html.tsx:28

    src/content/onboarding-copy.ts:86

    src/data/agent-catalog.ts:201

    src/lib/conversation-cache.ts:39

    src/lib/transcript.ts:66

    src/runtime/connection-config.ts:150

    src/runtime/daemon-provider.tsx:151

    src/runtime/daemon-provider.tsx:155

    src/runtime/daemon-provider.tsx:160

    src/styles/apply-appearance.ts:4

    src/styles/apply-appearance.ts:9

    src/styles/surface-styles.ts:25

  ⚠ Bugs: Data passed to parent via effect ×2
    Handing data back to a parent from a useEffect costs your
    users an extra render.
    → Fetch the data in the parent and pass it down as a prop
    (or return it from the hook), instead of handing it back up
    through a prop callback in a useEffect. See
    https://react.dev/learn/you-might-not-need-an-effect#passing-data-to-the-parent
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-pass-data-to-parent

    src/hooks/use-resizable-sidebar-width.ts:31-33

  ⚠ Bugs: Dynamic padding on contentContainerStyle ×6
    Your users see rows jump when a changing paddingBottom on
    contentContainerStyle shifts the whole list.
    → Use `contentInset={{ bottom: dynamicValue }}` so the OS
    shifts the content instead of relaying it out, which avoids
    the jump.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/rn-scrollview-dynamic-padding

    src/components/artifact/artifact-preview-panel.tsx:24

    src/components/artifact/artifact-preview-panel.tsx:43

    src/components/artifact/artifact-preview-panel.tsx:82

    src/components/layout/conversation-pane.tsx:573

    src/components/settings/settings-sidebar-nav.tsx:123

    src/screens/settings-screen.tsx:76

  ⚠ Maintainability: Pure function rebuilt every render
    `isNewlineKeyIntent` inside `Composer` uses no local state
    but is rebuilt on every render, so it wastes work & breaks
    memoized children. Move it to the top of the file, outside
    the component.
    → Move the function above the component, at the top of the
    file. It doesn't use local state, so rebuilding it each
    update is wasted work.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/prefer-module-scope-pure-function

    src/components/composer/composer.tsx:115

  ⚠ Maintainability: Large component is hard to read and change ×3
    Component "ConversationPane" is over 300 lines long, which
    is hard to read & change. Split it into a few smaller
    components.
    → Pull each section into its own component so the parent is
    easier to read, test, and change.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-giant-component

    src/components/layout/conversation-pane.tsx:83

    src/components/layout/home-pane.tsx:27

    src/components/sidebar/left-sidebar.tsx:37

  ⚠ Performance: Chained array iterations ×2
    This loops over your list twice because .filter().filter()
    makes two passes, so do it in one pass with .reduce() or a
    for...of loop
    → Combine `.map().filter()` style chains into one pass with
    `.reduce()` or a `for...of` loop, so you only loop over the
    list once
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/js-combine-iterations

    src/components/health/provider-catalog.tsx:88

    src/lib/message-text.ts:42

  ⚠ Maintainability: Non-component export in component file ×4
    This file exports non-components, so Fast Refresh can't
    safely preserve component state.
    → Move non-component exports out of component files so Fast
    Refresh can preserve component state instead of
    full-reloading.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/only-export-components

    src/components/composer/composer-attachment-menu.tsx:118

    src/components/composer/composer-chip.tsx:149

    src/components/transcript/message-actions.tsx:240

    src/runtime/daemon-provider.tsx:160

  ✖ Bugs: State updater has side effects ×14
    This state updater performs the nested state update
    "handleDrop()". React may run updater functions more than
    once, so side effects here can repeat or observe
    inconsistent external state.
    → Keep state updater callbacks pure and return only the
    next state. Move notifications, storage, timers, ref
    writes, and other external work into the event or effect
    that queues the update.
    Curl with no cache & follow the canonical fix and false positive check recipe before fixing: https://react.doctor/docs/rules/react-doctor/no-impure-state-updater

    src/components/composer/composer.tsx:98
    ┌──────────────────────────────────────────────────────────────┐
    │   97 |           onDragLeave: () => setDragActive(false),    │
    │ > 98 |           onDrop: (event: DragEvent) => void handleD[0m… │
    │      |                                                     [0m… │
    │   99 |         } as Record<string, unknown>)                 │
    └──────────────────────────────────────────────────────────────┘

    src/components/composer/model-picker-sheet.tsx:71
    ┌──────────────────────────────────────────────────────────────┐
    │   70 |       .then((rows) => {                               │
    │ > 71 |         setModels(rows);                              │
    │      |                   ^                                   │
    │   72 |         const current =                               │
    └──────────────────────────────────────────────────────────────┘

    src/components/layout/conversation-pane.tsx:302
    ┌──────────────────────────────────────────────────────────────┐
    │   301 |   const openForkFromMessage = useCallback((messageI[0m… │
    │ > 302 |     setForkSourceMessageId(messageId);               │
    │       |                            ^                         │
    │   303 |     setForkOpen(true);                               │
    └──────────────────────────────────────────────────────────────┘

    src/components/layout/home-pane.tsx:172
    ┌──────────────────────────────────────────────────────────────┐
    │   171 |   const handleHarnessSelect = useCallback((harnessI[0m… │
    │ > 172 |     setSelectedHarnessId(harnessId);                 │
    │       |                          ^                           │
    │   173 |     setSelectedModelId(undefined);                   │
    └──────────────────────────────────────────────────────────────┘

    src/components/layout/home-pane.tsx:178
    ┌──────────────────────────────────────────────────────────────┐
    │   177 |   const handleModelSelect = useCallback(async (mode[0m… │
    │ > 178 |     setSelectedModelId(modelId);                     │
    │       |                        ^                             │
    │   179 |   }, []);                                            │
    └──────────────────────────────────────────────────────────────┘

    src/components/onboarding/starter-screen.tsx:117
    ┌──────────────────────────────────────────────────────────────┐
    │   116 |           onDragLeave: () => setDragActive(false),   │
    │ > 117 |           onDrop: (event: DragEvent) => void handle[0m… │
    │       |                                                    [0m… │
    │   118 |         } as Record<string, unknown>)                │
    └──────────────────────────────────────────────────────────────┘

    src/components/orchestration/run-recipe-sheet.tsx:108
    ┌──────────────────────────────────────────────────────────────┐
    │   107 |   const selectRecipe = (id: string) => {             │
    │ > 108 |     setSelectedId(id);                               │
    │       |                   ^                                  │
    │   109 |     setInputsJson(INPUT_TEMPLATES[id] ?? "{}");      │
    └──────────────────────────────────────────────────────────────┘

    src/components/sidebar/fork-conversation-sheet.tsx:82
    ┌──────────────────────────────────────────────────────────────┐
    │   81 |       .then((rows) => {                               │
    │ > 82 |         setModels(rows);                              │
    │      |                   ^                                   │
    │   83 |         setSelectedModel(rows[0] ?? { id: "default",[0m… │
    └──────────────────────────────────────────────────────────────┘

    src/components/sidebar/new-conversation-sheet.tsx:80
    ┌──────────────────────────────────────────────────────────────┐
    │   79 |       .then((rows) => {                               │
    │ > 80 |         setModels(rows);                              │
    │      |                   ^                                   │
    │   81 |         setSelectedModel(rows[0] ?? { id: "default",[0m… │
    └──────────────────────────────────────────────────────────────┘

    src/components/ui/disclosure.tsx:28
    ┌──────────────────────────────────────────────────────────────┐
    │   27 |   const setOpen = (next: boolean) => {                │
    │ > 28 |     if (controlledOpen === undefined) setUncontrolle[0m… │
    │      |                                                     [0m… │
    │   29 |     onOpenChange?.(next);                             │
    └──────────────────────────────────────────────────────────────┘

    src/hooks/use-conversation.ts:83-85
    ┌──────────────────────────────────────────────────────────────┐
    │   82 |       if (activeConversationId.current !== id) retur[0m… │
    │ > 83 |       setConversation(dto);                           │
    │ > 84 |       setMessages(parsed);                            │
    │ > 85 |       setUiMessages(ui);                              │
    │   86 |       storeCachedConversation(id, {                   │
    └──────────────────────────────────────────────────────────────┘

    src/hooks/use-harness-providers.ts:43
    ┌──────────────────────────────────────────────────────────────┐
    │   42 |     async (agentId: string, enabled: boolean) => {    │
    │ > 43 |       setPendingId(agentId);                          │
    │      |                    ^                                  │
    │   44 |       try {                                           │
    └──────────────────────────────────────────────────────────────┘


  ────────────────────────────────────────────────────────────

  All 104 issues

  Security › 2 warnings
  Bugs › 14 errors, 46 warnings
  Performance › 9 warnings
  Accessibility › 5 warnings
  Maintainability › 28 warnings

Agent guidance
  - Treat React Doctor diagnostics as starting hypotheses. Read the relevant code before confirming or suppressing each finding.
  - For each group, decide true positive, false positive, or needs-human-review, then assign high/medium/low confidence.
  - Do not suppress a finding without evidence from the file in question. Confidence requires code context.
  - Understand the root cause before editing. Fix the underlying code instead of changing react-doctor config or suppressing rules unless explicitly asked.
  - Investigate deeply where relevant: race conditions, security-sensitive flows, state propagation, multi-file refactors, and downstream dependency chains.
  - Ignore pure style preferences, theoretical issues without real impact, missing features, and unrelated pre-existing code.
  - Start with high-confidence fixes that preserve behavior. Leave low-confidence or product-dependent changes as notes.
  - Run `npx react-doctor@latest --verbose --scope changed` before and after changes, plus relevant tests after each focused batch.
  - When available, spawn subagents or isolated worktrees for independent rule families, then review and merge only the best safe fixes.
  - Split unrelated, broad, or behavior-changing work into separate PRs/branches instead of one large cleanup.
  - When one rule spans dozens of files (a migration-scale change), fix a representative sample first, confirm the recipe holds, and get the code owner's sign-off before changing the rest. Don't mass-fix a broad pattern in one unreviewed pass.
  - For confirmed issues that cannot be fixed now, create GitHub issues with the rule, file/line, confidence, impact, and proposed fix.
  - If a fix needs an API, UX, or architecture decision, stop and ask before editing.

  ┌─────┐  49 / 100 Critical
  │ x x │  █████████████████████████▓▓▓░░░░░░░░░░░░░░░░░░░░░░
  │  ▽  │  React Doctor (https://react.doctor)
  └─────┘

  You could improve +6% by fixing the top 3 issues
  Full diagnostics written to /var/folders/_f/g4knpn213m33bf1cqqwm1kr80000gn/T/react-doctor-7b40f8da-7d10-4f29-8c1b-f3f8aa0eaf52

  ────────────────────────────────────────────────────────────

  Share: https://react.doctor/share?p=%40tamtri%2Fapp&s=49&e=14&w=90&f=48
  Tell others how you did on socials

  Docs: https://react.doctor/docs
  Learn more about fixing issues, setting up CI/CD, and
  configuring rules with a config file

  GitHub: https://github.com/millionco/react-doctor
  Report issues and star the repository!

