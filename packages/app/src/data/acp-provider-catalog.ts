/** Curated ACP catalog entries for the add-provider picker. Seeded from paseo. */

export interface AcpProviderCatalogEntry {
  id: string;
  title: string;
  description: string;
  version: string;
  installLink: string;
  command: readonly string[];
  env?: Readonly<Record<string, string>>;
}

export const ACP_PROVIDER_CATALOG: AcpProviderCatalogEntry[] = [
  {
    id: "gemini",
    title: "Gemini CLI",
    description: "Google's official CLI for Gemini",
    version: "0.49.0",
    installLink: "https://geminicli.com",
    command: ["npx", "-y", "@google/gemini-cli@0.49.0", "--acp"],
  },
  {
    id: "goose",
    title: "Goose",
    description: "Local, extensible open source AI agent for engineering tasks",
    version: "1.33.1",
    installLink: "https://block.github.io/goose/",
    command: ["goose", "acp"],
  },
  {
    id: "cursor",
    title: "Cursor",
    description: "Cursor's coding agent",
    version: "2026.03.30",
    installLink: "https://docs.cursor.com/en/cli/overview",
    command: ["cursor-agent", "acp"],
  },
  {
    id: "cline",
    title: "Cline",
    description: "Autonomous coding agent CLI with browser and file tools",
    version: "3.0.38",
    installLink: "https://cline.bot/cli",
    command: ["npx", "-y", "cline@3.0.38", "--acp"],
  },
  {
    id: "kilo",
    title: "Kilo",
    description: "The open source coding agent",
    version: "7.2.40",
    installLink: "https://kilo.ai/docs/code-with-ai/platforms/cli",
    command: ["kilo", "acp"],
  },
  {
    id: "qwen-code",
    title: "Qwen Code",
    description: "Alibaba's Qwen coding assistant",
    version: "0.19.7",
    installLink: "https://qwenlm.github.io/qwen-code-docs/en/users/overview",
    command: ["npx", "-y", "@qwen-code/qwen-code@0.19.7", "--acp", "--experimental-skills"],
  },
  {
    id: "hermes",
    title: "Hermes",
    description: "Nous Research self-improving AI agent",
    version: "manual",
    installLink: "https://hermes-agent.nousresearch.com/docs/user-guide/features/acp",
    command: ["hermes", "acp"],
  },
  {
    id: "opencode",
    title: "OpenCode",
    description: "Open source coding agent",
    version: "latest",
    installLink: "https://opencode.ai/docs",
    command: ["opencode", "acp"],
  },
  {
    id: "factory-droid",
    title: "Factory Droid",
    description: "AI coding agent powered by Factory AI",
    version: "0.164.1",
    installLink: "https://factory.ai/product/cli",
    command: ["npx", "-y", "droid@0.164.1", "exec", "--output-format", "acp-daemon"],
    env: {
      DROID_DISABLE_AUTO_UPDATE: "true",
      FACTORY_DROID_AUTO_UPDATE_ENABLED: "false",
    },
  },
  {
    id: "auggie",
    title: "Auggie CLI",
    description: "Augment Code's software agent with context engine",
    version: "0.32.0",
    installLink: "https://www.augmentcode.com/",
    command: ["npx", "-y", "@augmentcode/auggie@0.32.0", "--acp"],
    env: { AUGMENT_DISABLE_AUTO_UPDATE: "1" },
  },
];
