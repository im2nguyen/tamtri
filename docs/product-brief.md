# tamtri

**The VS Code of AI knowledge work. Any agent, any model, rendered richly. Open source.**

## The problem

The best agents live in a terminal. Ask one to turn a spreadsheet into a report and it prints "I created report.html." Then you go find the file yourself. Terminals cannot render. No charts, no dashboards, no forms, no interactive results. Even getting work in and out is awkward: you cannot drag a spreadsheet into a terminal, and everything that comes back is a path to go hunt down. Computing itself started in the terminal and moved to rich interfaces for a reason. Agents are retracing that history, and the people with the most to gain from them, the marketer, the analyst, the ops lead, the PM, will never live in a terminal anyway.

The apps with a real interface lock you in. Claude Cowork is Anthropic's engine. Codex is OpenAI's. You take the engine and the surface as a bundle, or you take neither.

## What tamtri is

One native Mac app. You pick the engine (Claude Code, Codex, Gemini, Goose, any ACP agent) and model when you create a conversation, then fork the conversation to try another. tamtri drives the chosen harness and renders everything it produces inline: the report, the chart, the interactive tool, the file it just wrote.

The demo that says it all: drag in a CSV, ask for a report, and the finished report renders right there in the conversation. Not a filename. The thing itself. Cowork at home, except the engine is your choice.

Trust is structural, not promised. Your conversations live as plain files in a folder you own. Open them in Finder. Sync them with your own iCloud, Dropbox, or Git. Fork one like a repo. No accounts, no telemetry, no cloud in the box. Walk away anytime and take everything with you.

## Why it's different

- **Engine and model agnostic.** Any harness, any model. Fork a conversation to try another. Never locked to one vendor.
- **Renders what a terminal can't.** Finished reports, charts, interactive apps, inline. This is the whole point.
- **Your data is files.** Legible, local, yours. Nothing hidden, nothing held hostage.
- **Native desktop + web UI.** Electron desktop and Expo web/mobile share one `@tamtri/app` codebase. Menu bar, shortcuts, and packaged Mac download are on the roadmap; dev builds use Electron today.

## How it works

Two open standards do the heavy lifting. The Agent Client Protocol connects tamtri to agents, so one integration unlocks dozens of them. The Model Context Protocol connects agents to tools and data, and tamtri sits in the middle as the gateway. Every tool call passes through one consent and audit point. tamtri holds your credentials and injects them downstream; the agent never sees a secret. You don't need to know any of this to use it. It means tamtri rides the ecosystem instead of fighting it.

## The bet

VS Code did not win by being the best editor. It won by being open, neutral, and the best host of an open standard. The ecosystem grew on top of it, and the place became worth more than any feature.

tamtri is that bet for AI knowledge work, and the timing is the point. MCP formalized elicitation and tasks in late 2025; interactive Apps became its first official extension in January 2026. These primitives are becoming table stakes for every capable model. Somebody will build the surface that hosts them best. It should be open, and owned by no vendor.

## Built to be contributed to

This is a young codebase designed for other people's hands. A portable Rust core with Expo/Electron surfaces today (native Swift shell planned). Layer boundaries are sacred: the core never touches UI, and no adapter leaks its quirks past its own seam, because the adapter interface is the future plugin contract. Architectural decisions are written in [docs/tamtri-decisions.md](./tamtri-decisions.md); live docs are in [docs/README.md](./README.md).

**Use it:** download or build from source, set up an agent app, run the report-from-data demo.
**Build it:** read `tamtri-decisions.md`, pick an area from [product-gaps.md](./product-gaps.md), open an issue to claim it.

---

*Named for the Vietnamese "tâm trí": the mind. tamtri.ai*
