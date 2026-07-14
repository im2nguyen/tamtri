# Getting started with tamtri

tamtri is for people who want agent tools (Claude Code, Codex, Hermes, and others) in a **normal app** — not a terminal. Your conversations are plain files you own under `~/.tamtri/vault`.

**New here?** Start with [quickstart.md](./quickstart.md) for the download → first artifact path.

## Concepts (30 seconds)

| Term | Plain language |
|------|----------------|
| **Agent app** (harness) | The tool that does the work: Claude Code, Codex, Hermes, Goose, etc. You pick one per conversation. |
| **Model** | Which AI version the agent app uses (e.g. Sonnet, GPT). Set when you start; to change later, copy the conversation to a new branch (fork). |
| **Conversation** | Your saved thread — like a document folder with messages, files, and reports. |
| **Vault** | The folder on your Mac where all conversations live. Open it in Finder anytime. |

tamtri **detects and guides**; it does not install agent apps for you. If Claude Code is missing, tamtri links to install instructions.

## Install tamtri (developers today)

Packaged Mac downloads are coming. Today:

1. Install **Rust**, **Node.js 20+**, and (optional) **Expo Go** on your phone for mobile dev.
2. Clone the repo and build:

```bash
pnpm install
pnpm run daemon:build
```

3. Run the desktop app (recommended):

```bash
pnpm run dev:desktop
```

Or the browser:

```bash
pnpm run dev:web
# Open http://localhost:8081
```

Electron spawns the daemon automatically. Browser mode starts the daemon via the dev script.

Runtime files: `~/.tamtri/` (port, token, vault, credentials).

## Set up agent apps

1. Open **Agents & providers** in the sidebar (or go to `/health`).
2. Each row shows **Available**, **Not installed**, or **Disabled**.
3. For missing apps, click **Install guide** — opens the vendor docs (Anthropic, OpenAI, etc.).
4. After installing on your Mac, click **Refresh**.
5. Enable the agents you want with the toggle.

**Common agent apps:**

| Name | Typical install |
|------|-----------------|
| Claude Code | [Anthropic Claude Code docs](https://docs.anthropic.com/en/docs/claude-code) |
| Codex | OpenAI Codex CLI |
| Hermes | [Nous Hermes](https://github.com/NousResearch/hermes) |
| Goose | [Block Goose](https://block.github.io/goose/) |
| OpenCode | [opencode.ai](https://opencode.ai/docs) |

Send the **IT checklist** at the bottom of Agents & providers to your admin if they provision tools for you.

## Start a conversation

1. Click **New conversation** in the sidebar.
2. Choose an **agent app** and **model** (only apps marked Available work).
3. Type a message or drop a file (CSV, document) into the composer.
4. Example: *"Turn this CSV into a self-contained HTML report."*

When the agent writes a report, it appears in the **Artifacts** panel on the right — click the card to preview.

## Mobile (same Wi-Fi)

1. `pnpm run dev:ios` from the repo root (Mac and iPhone on the same network).
2. Scan the QR code in **Expo Go** (SDK 54).
3. If needed: sidebar → **Connect host** → paste URL and token from the terminal.

Remote access without LAN is not available yet (relay in progress).

## Where your data lives

```
~/.tamtri/vault/conversations/<date>-<title>--<id>/
  meta.json          settings for this conversation
  messages.jsonl     full transcript (one message per line)
  attachments/       reports and files shown in the UI
  workdir/           working files the agent used
```

Export a conversation from the header menu as a `.tamtri` bundle to share.

## Next steps

- [quickstart.md](./quickstart.md) — first launch through your first report
- [product-brief.md](./product-brief.md) — why tamtri exists
- [provider-adapters.md](./provider-adapters.md) — which agent apps are supported
- [orchestration.md](./orchestration.md) — multi-step recipes (handoff, committee)
- [README.md](../README.md) — developer commands
