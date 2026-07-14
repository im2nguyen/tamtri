/** Curated agent catalog for the add-agent picker. Ids match vault roster entries. */

export type AgentAdapterKind =
  | "acp"
  | "claude_native"
  | "codex_native"
  | "opencode_native"
  | "pi_native";

export interface AgentCatalogEntry {
  id: string;
  title: string;
  description: string;
  installLink: string;
  installSteps: string;
  command: string;
  args: string[];
  adapter: AgentAdapterKind;
  env?: { name: string; value: string }[];
  featured?: boolean;
}

export const AGENT_CATALOG: AgentCatalogEntry[] = [
  {
    id: "claude-native",
    title: "Claude Code",
    description: "Anthropic's agent for knowledge work and coding (native transport).",
    installLink: "https://docs.anthropic.com/en/docs/claude-code",
    installSteps:
      "Install the Claude Code CLI, then run `claude login` in Terminal. After auth succeeds, add Claude here and enable it.",
    command: "claude",
    args: [],
    adapter: "claude_native",
    featured: true,
  },
  {
    id: "codex-native",
    title: "Codex",
    description: "OpenAI Codex via the native app-server transport.",
    installLink: "https://developers.openai.com/codex",
    installSteps:
      "Install the Codex CLI and sign in with your OpenAI account. Add Codex here once `codex` is on your PATH.",
    command: "codex",
    args: ["app-server"],
    adapter: "codex_native",
    featured: true,
  },
  {
    id: "opencode-native",
    title: "OpenCode",
    description: "Open-source coding agent (native HTTP server).",
    installLink: "https://opencode.ai/docs",
    installSteps:
      "Install OpenCode and configure at least one model provider. Add OpenCode here once `opencode` is on your PATH.",
    command: "opencode",
    args: ["serve"],
    adapter: "opencode_native",
    featured: true,
  },
  {
    id: "pi-native",
    title: "Pi",
    description: "Pi agent via native RPC mode.",
    installLink: "https://github.com/badlogic/pi-mono",
    installSteps: "Install Pi and ensure `pi` is on your PATH, then add it here.",
    command: "pi",
    args: [],
    adapter: "pi_native",
    featured: true,
  },
  {
    id: "hermes-acp",
    title: "Hermes",
    description: "General-purpose self-improving agent from Nous Research (ACP).",
    installLink: "https://hermes-agent.nousresearch.com/docs/user-guide/features/acp",
    installSteps:
      "Install Hermes Agent and confirm `hermes acp` works in Terminal. Hermes is often installed to `~/.local/bin/hermes`.",
    command: "hermes",
    args: ["acp"],
    adapter: "acp",
    featured: true,
  },
  {
    id: "goose-acp",
    title: "Goose",
    description: "Local, extensible open-source agent for engineering tasks (ACP).",
    installLink: "https://block.github.io/goose/docs/getting-started/installation/",
    installSteps: "Install Goose and confirm `goose` runs on your PATH, then add it here.",
    command: "goose",
    args: [],
    adapter: "acp",
    featured: true,
  },
  {
    id: "claude-code-acp",
    title: "Claude Code (ACP)",
    description: "Claude Code via the Agent Client Protocol fallback.",
    installLink: "https://docs.anthropic.com/en/docs/claude-code",
    installSteps: "Install Claude Code CLI, run `claude login`, then add this ACP entry.",
    command: "claude",
    args: ["acp"],
    adapter: "acp",
  },
  {
    id: "opencode-acp",
    title: "OpenCode (ACP)",
    description: "OpenCode via ACP instead of the native server.",
    installLink: "https://opencode.ai/docs",
    installSteps: "Install OpenCode, then add this entry if you prefer ACP over the native adapter.",
    command: "opencode",
    args: ["acp"],
    adapter: "acp",
  },
  {
    id: "pi-acp",
    title: "Pi (ACP bridge)",
    description: "Pi via the community pi-acp bridge.",
    installLink: "https://github.com/svkozak/pi-acp",
    installSteps: "Install the pi-acp bridge and ensure `pi-acp` is on your PATH.",
    command: "pi-acp",
    args: [],
    adapter: "acp",
  },
  {
    id: "gemini",
    title: "Gemini CLI",
    description: "Google's official CLI for Gemini (ACP).",
    installLink: "https://geminicli.com",
    installSteps: "Follow Google's install guide, then add Gemini here.",
    command: "npx",
    args: ["-y", "@google/gemini-cli@0.49.0", "--acp"],
    adapter: "acp",
  },
  {
    id: "cursor",
    title: "Cursor",
    description: "Cursor's coding agent (ACP).",
    installLink: "https://docs.cursor.com/en/cli/overview",
    installSteps: "Install the Cursor CLI agent, then add it here.",
    command: "cursor-agent",
    args: ["acp"],
    adapter: "acp",
  },
  {
    id: "cline",
    title: "Cline",
    description: "Autonomous coding agent CLI with browser and file tools.",
    installLink: "https://cline.bot/cli",
    installSteps: "Install Cline CLI, then add it here.",
    command: "npx",
    args: ["-y", "cline@3.0.38", "--acp"],
    adapter: "acp",
  },
  {
    id: "kilo",
    title: "Kilo",
    description: "The open source coding agent.",
    installLink: "https://kilo.ai/docs/code-with-ai/platforms/cli",
    installSteps: "Install Kilo CLI, then add it here.",
    command: "kilo",
    args: ["acp"],
    adapter: "acp",
  },
  {
    id: "qwen-code",
    title: "Qwen Code",
    description: "Alibaba's Qwen coding assistant.",
    installLink: "https://qwenlm.github.io/qwen-code-docs/en/users/overview",
    installSteps: "Install Qwen Code CLI, then add it here.",
    command: "npx",
    args: ["-y", "@qwen-code/qwen-code@0.19.7", "--acp", "--experimental-skills"],
    adapter: "acp",
  },
  {
    id: "factory-droid",
    title: "Factory Droid",
    description: "AI coding agent powered by Factory AI.",
    installLink: "https://factory.ai/product/cli",
    installSteps: "Install Factory Droid CLI, then add it here.",
    command: "npx",
    args: ["-y", "droid@0.164.1", "exec", "--output-format", "acp-daemon"],
    adapter: "acp",
    env: [
      { name: "DROID_DISABLE_AUTO_UPDATE", value: "true" },
      { name: "FACTORY_DROID_AUTO_UPDATE_ENABLED", value: "false" },
    ],
  },
  {
    id: "auggie",
    title: "Auggie CLI",
    description: "Augment Code's software agent with context engine.",
    installLink: "https://www.augmentcode.com/",
    installSteps: "Install Auggie CLI, then add it here.",
    command: "npx",
    args: ["-y", "@augmentcode/auggie@0.32.0", "--acp"],
    adapter: "acp",
    env: [{ name: "AUGMENT_DISABLE_AUTO_UPDATE", value: "1" }],
  },
];

export function catalogEntryById(id: string): AgentCatalogEntry | undefined {
  return AGENT_CATALOG.find((entry) => entry.id === id);
}
