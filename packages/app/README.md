# @tamtri/app

The single tamtri UI — Expo + React Native Web, Paseo-inspired dark theme with green accent.

One codebase renders on **desktop** (Electron renderer), **web**, and **mobile** (later).

## Design

Borrowed from [Paseo](https://github.com/nousresearch/paseo):

- Layered surfaces (`surfaceSidebar` #141716, `surface0` #181B1A, green accent #20744A)
- Left sidebar + centered transcript (max 820px) + bottom composer
- Status dots on conversation rows, tool/thinking cards in the transcript

## Dev (desktop)

```bash
# Terminal 1 — Expo web dev server
npm run app:web

# Terminal 2 — Electron shell (loads Metro via IPC bridge)
npm run desktop:dev
```

Electron spawns `tamtri-daemon` and bridges the wire protocol; the app never sees the bearer token.

## Dev (web-only, direct WS)

Set `EXPO_PUBLIC_DAEMON_WS_URL` and optionally `EXPO_PUBLIC_DAEMON_TOKEN`, then `npm run web`.

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
