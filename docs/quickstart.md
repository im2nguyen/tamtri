# Quickstart: first report in tamtri

This guide gets you from download to your first inline report — no terminal required.

## 1. Open tamtri

Download and open tamtri on your Mac. On first launch you see a short welcome flow.

If tamtri cannot connect, quit completely and reopen the app once.

## 2. Set up an agent app

tamtri works with agent apps you install separately (Claude Code, Codex, Goose, Hermes, and others).

1. Follow the setup screen to install one recommended app, or open **Agents & providers** from the sidebar.
2. Each row shows **Available**, **Not installed**, or **Disabled**.
3. Click **Install guide** for vendor instructions, then **Refresh** after installing.
4. If IT provisions tools for you, copy the checklist on the setup screen and send it to your admin.

You need at least one **Available** agent app before running a conversation.

## 3. Run the sample report

After an agent app is ready:

1. The starter screen offers bundled sample sales data.
2. Review the prefilled prompt — nothing runs until you press **Run sample**.
3. Watch the transcript as the agent works. When a report is ready, tamtri opens the **Artifacts** panel on the right automatically.

The sample produces a self-contained HTML report you can read inside tamtri.

## 4. Try your own file

When the sample finishes, tamtri prompts you to drop in your own CSV or document:

1. Drag a file into the composer, or use the **+** attachment menu.
2. Ask for a report (for example: *"Turn this CSV into a self-contained HTML report."*).
3. Approve permissions when asked — you choose allow once, for this conversation, or for a folder.

Your conversations are saved as plain files under `~/.tamtri/vault`.

## 5. What to explore next

- **New conversation** — pick a different agent app or model (fixed per thread; fork to switch).
- **Export** — save a `.tamtri` bundle from the conversation header.
- **Agents & providers** — enable more agent apps or add a custom connection under Advanced.

For developer install steps, see [getting-started.md](./getting-started.md).
