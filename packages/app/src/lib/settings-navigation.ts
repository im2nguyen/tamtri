export const SETTINGS_SECTION_IDS = [
  "general",
  "appearance",
  "providers",
  "usage",
  "connect",
  "import",
  "sessions",
  "advanced",
] as const;

export type SettingsSectionSlug = (typeof SETTINGS_SECTION_IDS)[number];
export type SettingsNavGroupId = "app" | "agents" | "connection" | "data" | "advanced";

export interface SettingsNavItem {
  id: SettingsSectionSlug;
  group: SettingsNavGroupId;
  label: string;
  description: string;
  icon:
    | "general"
    | "appearance"
    | "providers"
    | "usage"
    | "connect"
    | "import"
    | "sessions"
    | "advanced";
}

export const SETTINGS_NAV_GROUPS: ReadonlyArray<{ id: SettingsNavGroupId; label: string }> = [
  { id: "app", label: "App" },
  { id: "agents", label: "Agents" },
  { id: "connection", label: "Connection" },
  { id: "data", label: "Data" },
  { id: "advanced", label: "Advanced" },
];

export const SETTINGS_NAV_ITEMS: readonly SettingsNavItem[] = [
  {
    id: "general",
    group: "app",
    label: "General",
    description: "Set interface density and other app-wide defaults.",
    icon: "general",
  },
  {
    id: "appearance",
    group: "app",
    label: "Appearance",
    description: "Customize theme, fonts, and syntax highlighting.",
    icon: "appearance",
  },
  {
    id: "providers",
    group: "agents",
    label: "Providers",
    description: "Manage agent apps, readiness, authentication, and picker visibility.",
    icon: "providers",
  },
  {
    id: "usage",
    group: "agents",
    label: "Usage",
    description: "Review quota and credit usage reported by signed-in providers.",
    icon: "usage",
  },
  {
    id: "connect",
    group: "connection",
    label: "Connect host",
    description: "Connect a phone or tablet to your tamtri daemon.",
    icon: "connect",
  },
  {
    id: "import",
    group: "data",
    label: "Import bundle",
    description: "Open a .tamtri bundle or conversation folder.",
    icon: "import",
  },
  {
    id: "sessions",
    group: "data",
    label: "Import sessions",
    description: "Bring Claude and Codex terminal sessions into your vault.",
    icon: "sessions",
  },
  {
    id: "advanced",
    group: "advanced",
    label: "Diagnostics & vault",
    description: "Review vault diagnostics reported by the daemon.",
    icon: "advanced",
  },
];

export function isSettingsSection(value: unknown): value is SettingsSectionSlug {
  return typeof value === "string" && (SETTINGS_SECTION_IDS as readonly string[]).includes(value);
}

export function normalizeSettingsSection(value: unknown): SettingsSectionSlug {
  return isSettingsSection(value) ? value : "general";
}

const SETTINGS_PATH_PREFIX = "/settings/";

export function sectionFromPathname(pathname: string): SettingsSectionSlug {
  const path = pathname.split(/[?#]/, 1)[0] ?? pathname;
  if (!path.startsWith(SETTINGS_PATH_PREFIX)) return "general";
  const rest = path.slice(SETTINGS_PATH_PREFIX.length);
  const [rawSection] = rest.split("/");
  return normalizeSettingsSection(rawSection);
}

export function settingRowAnchorId(title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `setting-${slug}`;
}
