# @tamtri/app

The single tamtri UI — Expo + React Native Web, Paseo-inspired dark theme with green accent.

One codebase renders on **desktop** (Electron renderer), **web**, and **mobile** (later).

**Dev commands and the full monorepo command reference:** see the [root README](../../README.md).

## Design

Borrowed from [Paseo](https://github.com/nousresearch/paseo):

- Layered surfaces (`surfaceSidebar` #141716, `surface0` #181B1A, green accent #20744A)
- Left sidebar + centered transcript (max 820px) + bottom composer
- Status dots on conversation rows, tool/thinking cards in the transcript

## Dev

Use the root scripts (from the repo root):

```bash
npm run dev:web       # browser: daemon + Metro + auth
npm run dev:desktop     # Electron: Metro + shell (daemon spawned by Electron)
```

See [README.md](../../README.md) for manual split workflows, build commands, and environment variables.

## Structure

```
src/
  app/              Expo Router routes
  components/       sidebar, transcript, composer, ui
  runtime/          DaemonClient provider
  desktop/          Electron IPC transport
  styles/           Paseo-inspired tokens
  hooks/            conversation list/detail
```

## Connectivity

The UI constructs a `DaemonClient` from `@tamtri/client`:

- **Desktop:** `window.tamtri.transport` (Electron IPC → main process → daemon)
- **Web:** direct localhost WebSocket (when configured)
- **Mobile (later):** relay E2EE channel after pairing
