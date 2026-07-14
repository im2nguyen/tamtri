/** Plain-language first-run copy. Uses "agent app", not harness. */

export const onboardingCopy = {
  welcome: {
    title: "Welcome to tamtri",
    subtitle:
      "Turn spreadsheets and documents into reports you can read right here — no terminal required.",
    body: "tamtri connects to agent apps you already have (Claude Code, Codex, Goose, and others). Your conversations stay as plain files you own.",
    continueLabel: "Get started",
    skipLabel: "Skip for now",
  },
  home: {
    title: "What should we work on?",
    subtitle: "Drop a file, ask a question, or pick up where you left off.",
    composerPlaceholder: "Turn this CSV into a report, summarize a doc…",
    tourLabel: "Take the guided tour",
    noAgentsTitle: "Set up an agent app first",
    noAgentsBody: "Install Claude Code, Hermes, or another agent app, then come back here to start.",
    setupAgentsLabel: "Set up agents",
  },
  gate: {
    title: "Set up an agent app",
    subtitle: "tamtri needs one installed agent app before you can run a conversation.",
    recommendedTitle: "Recommended for your first report",
    installGuideLabel: "Install guide",
    refreshLabel: "Check again",
    continueLabel: "Continue",
    itTitle: "Need IT to install this?",
    itBody: "Copy the checklist below and send it to your admin. tamtri detects apps on this Mac — it does not install them for you.",
    itCopyLabel: "Copy checklist",
    itSentLabel: "Checklist copied",
    noAgentsTitle: "No agent apps ready yet",
    noAgentsBody: "Install one of the apps below, then tap Check again.",
    advancedLink: "All agent apps & settings",
  },
  starter: {
    title: "Try the report demo",
    subtitle: "Drop a CSV or use our sample data. You choose when to run — nothing starts automatically.",
    dropHint: "Drop a CSV here, or use the sample below",
    dropActive: "Drop to attach…",
    sampleTitle: "Sample: quarterly sales",
    sampleHint: "A small CSV bundled for your first run",
    promptLabel: "Prompt",
    runSampleLabel: "Run sample",
    runningLabel: "Starting…",
    attachOwnHint: "Or attach your own file with the + button after you continue",
  },
  payoff: {
    dropYoursTitle: "Now drop in one of yours",
    dropYoursBody:
      "Drag a CSV or document into the composer, ask for a report, and tamtri will show the result in the panel on the right.",
    dismissLabel: "Got it",
  },
  example: {
    badge: "Pre-recorded",
    cardTitle: "This is a pre-recorded example",
    cardBody: "Browse the transcript to see how a report conversation looks. Copy it to start your own thread with the same context.",
    useLabel: "Use this example",
    usingLabel: "Copying…",
    dismissLabel: "Dismiss",
  },
  permissionCoach: {
    title: "You're in control",
    body: "Every file edit and command needs your approval. Choose allow once, for this conversation, or for this folder — deny is always available.",
    dismissLabel: "Got it",
  },
} as const;

export const SAMPLE_CSV_FILENAME = "sales.csv";

export const SAMPLE_CSV = `month,revenue,region
Jan,12000,North
Feb,14500,North
Mar,13200,North
Jan,9800,South
Feb,11200,South
Mar,12800,South
Jan,7600,West
Feb,8100,West
Mar,9400,West`;

export const SAMPLE_PROMPT =
  "Turn the attached sales.csv into a self-contained HTML report with a summary table and totals by region. The report must work offline with no external scripts or CDN links.";

/** Preferred agent ids for client-side fallback only; server owns recommendation policy. */
export const RECOMMENDED_AGENT_IDS = [
  "claude-native",
  "claude-code-acp",
  "codex-native",
  "hermes-acp",
  "goose-acp",
] as const;
