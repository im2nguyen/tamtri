import {
  settingRowAnchorId,
  SETTINGS_NAV_ITEMS,
  type SettingsSectionSlug,
} from "./settings-navigation";

export interface SettingsSearchEntry {
  id: string;
  section: SettingsSectionSlug;
  title: string;
  keywords: string;
  target?: string | null;
}

export const SETTINGS_SEARCH_ENTRIES: readonly SettingsSearchEntry[] = [
  {
    id: "general:density",
    section: "general",
    title: "Interface density",
    keywords: "compact comfortable spacious rows gutters composer spacing",
  },
  {
    id: "appearance:theme",
    section: "appearance",
    title: "Appearance",
    keywords: "theme dark light system",
  },
  {
    id: "appearance:fonts",
    section: "appearance",
    title: "Interface font",
    keywords: "font family size typography code monospace",
  },
  {
    id: "appearance:syntax",
    section: "appearance",
    title: "Highlight theme",
    keywords: "syntax code highlighting color",
  },
  {
    id: "providers:agents",
    section: "providers",
    title: "Agent apps",
    keywords: "provider enable visible readiness install sign in auth claude codex opencode pi hermes acp",
    target: null,
  },
  {
    id: "providers:add",
    section: "providers",
    title: "Add agent",
    keywords: "catalog register provider cli command",
    target: null,
  },
  {
    id: "usage:quota",
    section: "usage",
    title: "Provider usage",
    keywords: "quota credits limits billing plan remaining",
    target: null,
  },
  {
    id: "connect:host",
    section: "connect",
    title: "Host connection",
    keywords: "websocket lan relay token phone mobile daemon pairing",
    target: null,
  },
  {
    id: "import:bundle",
    section: "import",
    title: "Import bundle",
    keywords: "tamtri conversation folder attachments integrity",
    target: null,
  },
  {
    id: "sessions:native",
    section: "sessions",
    title: "Native sessions",
    keywords: "claude codex terminal history import",
    target: null,
  },
  {
    id: "advanced:vault",
    section: "advanced",
    title: "Vault diagnostics",
    keywords: "issues repair duplicate malformed storage refresh",
    target: null,
  },
];

const SECTION_LABELS = new Map(SETTINGS_NAV_ITEMS.map((item) => [item.id, item.label]));

export function settingsSectionLabel(section: SettingsSectionSlug): string {
  return SECTION_LABELS.get(section) ?? section;
}

export function settingsSearchEntryTarget(entry: SettingsSearchEntry): string | null {
  return entry.target === undefined ? settingRowAnchorId(entry.title) : entry.target;
}

function matchScore(entry: SettingsSearchEntry, terms: readonly string[]): number | null {
  const title = entry.title.toLowerCase();
  const section = settingsSectionLabel(entry.section).toLowerCase();
  const keywords = entry.keywords.toLowerCase();
  let score = 0;

  for (const term of terms) {
    if (title === term) score += 120;
    else if (title.startsWith(term)) score += 80;
    else if (title.includes(term)) score += 55;
    else if (section.includes(term)) score += 30;
    else if (keywords.includes(term)) score += 15;
    else return null;
  }
  return score;
}

export function rankSettingsSearchEntries(
  query: string,
  limit = 12,
): readonly SettingsSearchEntry[] {
  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  if (terms.length === 0) return [];

  return SETTINGS_SEARCH_ENTRIES.map((entry, index) => ({
    entry,
    index,
    score: matchScore(entry, terms),
  }))
    .filter((row): row is { entry: SettingsSearchEntry; index: number; score: number } =>
      row.score !== null
    )
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .slice(0, limit)
    .map((row) => row.entry);
}
