# @tamtri/app

The single tamtri UI — Expo + React Native Web (SDK 54), dark theme with green accent.

One codebase renders on **desktop** (Electron renderer), **web**, and **mobile** (iOS/Android via Expo Go).

**User guide:** [docs/getting-started.md](../../docs/getting-started.md)
**Dev commands:** [root README](../../README.md)

## Design

- Layered surfaces (sidebar, transcript, composer, artifact sidebar)
- Resizable left and right sidebars (desktop)
- Paseo-inspired layout patterns; tamtri-owned theme tokens (`styles/themes/`)

## Dev

```bash
pnpm run dev:web       # browser: daemon + Metro + auth
pnpm run dev:desktop   # Electron: Metro + shell (daemon spawned by Electron)
pnpm run dev:ios       # physical iPhone: daemon on LAN + Expo Go (same Wi-Fi)
```

See [README.md](../../README.md) for manual workflows and environment variables.

### Physical iPhone (LAN dev)

1. `pnpm run daemon:build`
2. Same Wi-Fi as Mac
3. `pnpm run dev:ios` → scan QR in Expo Go (SDK 54)
4. Fallback: sidebar → **Connect host**

Relay remote access is not production-ready yet.

## Structure

```
src/
  app/              Expo Router routes (conversations, health, settings, onboarding)
  components/
    sidebar/        conversation list, new/fork sheets
    transcript/     message list, tool/artifact cards, message actions
    composer/       prompt input, harness/model chips, attachments
    artifact/       right-sidebar preview panel
    health/         agents & providers, usage, catalog
    layout/         app shell, conversation pane
  hooks/            daemon data (conversations, agents, providers, appearance)
  lib/              transcript, ai-sdk-bridge, ui-message streaming
  runtime/          DaemonClient provider, mobile connection config
  desktop/          Electron IPC transport
  styles/           theme tokens, light/dark, appearance store
  content/          user-facing copy (onboarding, when added)
```

## Connectivity

- **Desktop:** `window.tamtri.transport` (Electron IPC → daemon)
- **Web:** WebSocket + bearer token (`dev:web` injects env)
- **Mobile:** LAN WebSocket or Connect host screen

The UI never reads the vault filesystem directly; all state comes from the daemon wire protocol.
