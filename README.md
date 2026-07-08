# tamtri

Open-source, model-agnostic agent UI shell. A Rust **daemon** owns the vault, MCP gateway, and harness processes; every surface (Electron desktop, web, mobile later) is a thin client over the wire protocol.

Hero use case: **report from data, not code** — turn a CSV into an inline report instead of a terminal saying "I created report.html."

License: **AGPL-3.0-or-later**. See [CLAUDE.md](./CLAUDE.md) and [docs/](./docs/) for architecture and milestones.

## Prerequisites

- **Rust** 1.89+ (daemon + core)
- **Node.js** 20+ and **npm** (TypeScript surfaces)
- At least one harness installed locally (e.g. Claude Code, Codex) for real runs

```bash
npm install
npm run daemon:build    # compile tamtri-daemon (once, or after core changes)
```

Runtime files live under `~/.tamtri/` (daemon port, auth token, vault, sealed credentials).

## Quick start

```bash
# Browser — daemon + Metro + auth env in one command
npm run dev:web
# Open http://localhost:8081

# Desktop — Metro + Electron (Electron spawns the daemon, IPC bridge — no token in the renderer)
npm run dev:desktop
```

Build the daemon first if you have not already: `npm run daemon:build`.

## Commands

### Development (recommended)

| Command | What it does |
|---------|----------------|
| `npm run dev:web` | Spawns `tamtri-daemon` on port 8377, reads `~/.tamtri/daemon.token`, starts Expo web with `EXPO_PUBLIC_DAEMON_*` set |
| `npm run dev:desktop` | Starts Metro, then Electron with `TAMTRI_USE_DEV_SERVER=1` (loads `http://localhost:8081`) |

### Development (manual / split)

| Command | What it does |
|---------|----------------|
| `npm run app:web` | Metro only (`expo start --web` on `:8081`). Browser mode still needs a running daemon — prefer `dev:web` |
| `npm run desktop:dev` | Electron only. Expects Metro already on `:8081`. Electron spawns the daemon and bridges IPC |
| `npm run daemon:run` | Run `tamtri-daemon` in the foreground (default port 8377, or ephemeral if `TAMTRI_PORT=0`) |

**Manual web + daemon:** run `npm run daemon:run`, copy the token from `~/.tamtri/daemon.token`, set `EXPO_PUBLIC_DAEMON_WS_URL=ws://127.0.0.1:<port>/ws` and `EXPO_PUBLIC_DAEMON_TOKEN=<token>`, then `npm run app:web`.

**Manual desktop (two terminals):**

```bash
npm run app:web       # terminal 1
npm run desktop:dev   # terminal 2
```

### Build

| Command | What it does |
|---------|----------------|
| `npm run daemon:build` | `cargo build -p tamtri-daemon` |
| `npm run desktop:build` | Export Expo web bundle + bundle Electron (`packages/desktop/dist/`) |
| `npm run build` | Build all npm workspaces that define a `build` script |
| `npm run typecheck` | Typecheck all npm workspaces |

### Protocol / core

| Command | What it does |
|---------|----------------|
| `npm run protocol:generate` | Regenerate `packages/protocol/src/generated.ts` from Rust typeshare annotations |
| `cargo build` | Build the Rust workspace |
| `cargo test` | Run core + daemon tests |
| `cargo clippy --all-targets -- -D warnings` | Lint Rust (required before merge) |

### Environment overrides

| Variable | Purpose |
|----------|---------|
| `TAMTRI_HOME` | Daemon runtime dir (default `~/.tamtri`) |
| `TAMTRI_PORT` | Daemon listen port (`8377` default; `0` = ephemeral) |
| `TAMTRI_DAEMON_BIN` | Path to `tamtri-daemon` binary (Electron uses this in dev) |
| `TAMTRI_USE_DEV_SERVER` | Electron loads Metro instead of the static export |
| `TAMTRI_DEV_URL` | Override Metro URL (default `http://localhost:8081`) |
| `EXPO_PUBLIC_DAEMON_WS_URL` | Web client WebSocket URL |
| `EXPO_PUBLIC_DAEMON_TOKEN` | Web client bearer token (from `~/.tamtri/daemon.token`) |

## Repo layout

```
/core              TamtriCore: vault, MCP gateway, harness adapters, wire protocol
/daemon            tamtri-daemon binary (axum WebSocket server)
/packages
  /protocol        typeshare-generated wire types
  /client          DaemonClient SDK
  /relay           E2E relay crypto
  /app             Expo + React Native Web UI
  /desktop         Electron shell (spawns daemon, IPC bridge)
/docs              Architecture, protocol, milestones
```

## Surfaces and connectivity

- **Desktop (Electron):** `window.tamtri.transport` → main process → localhost WebSocket → daemon. Token never enters the renderer.
- **Web (browser):** Direct WebSocket to the daemon with bearer token (wired automatically by `dev:web`).
- **Mobile (later):** Relay E2EE channel after pairing.

## Docs

- [daemon-protocol.md](./docs/daemon-protocol.md) — wire protocol
- [provider-adapters.md](./docs/provider-adapters.md) — Claude/Codex native + ACP fallback
- [vault-format.md](./docs/vault-format.md) — legible conversation storage
- [packages/app/README.md](./packages/app/README.md) — UI structure and design notes
