# @tamtri/app

The single tamtri UI — Expo + React Native Web, Paseo-inspired dark theme with green accent.

One codebase renders on **desktop** (Electron renderer), **web**, and **mobile** (later).

## Design

Borrowed from [Paseo](https://github.com/nousresearch/paseo):

- Layered surfaces (`surfaceSidebar` #141716, `surface0` #181B1A, green accent #20744A)
- Left sidebar + centered transcript (max 820px) + bottom composer
- Status dots on conversation rows, tool/thinking cards in the transcript

## Dev (recommended)

```bash
# Browser — one command: daemon + Metro + auth env
npm run dev:web

# Desktop — one command: Metro + Electron (Electron spawns the daemon)
npm run dev:desktop
```

Build the daemon once if you have not already: `npm run daemon:build`.

### Manual split (two terminals)

```bash
npm run app:web          # Metro only — browser still needs a daemon; prefer dev:web
npm run desktop:dev      # Electron only — needs Metro already on :8081
```

Electron spawns `tamtri-daemon` and bridges the wire protocol over IPC; the renderer never sees the bearer token.

## Dev (web-only, manual env)

If you run the daemon yourself (`npm run daemon:run`), set `EXPO_PUBLIC_DAEMON_WS_URL` and `EXPO_PUBLIC_DAEMON_TOKEN` (from `~/.tamtri/daemon.token`), then `npm run app:web`.

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
