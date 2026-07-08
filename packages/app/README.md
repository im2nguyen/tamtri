# @tamtri/app

The single tamtri UI, built with Expo + React Native Web so one codebase renders
on desktop (as the Electron renderer), web, and mobile (iOS/Android).

## Planned structure (paseo-informed)

```
src/
  app/           Expo Router routes (_layout, index, sessions, pair-scan, settings)
  components/    shared UI primitives (cards, composer, sidebar, transcript)
  screens/       full-screen compositions
  stores/        zustand stores (session, panels, hosts) + react-query for server data
  styles/        react-native-unistyles themes + design tokens
  runtime/       host registry; builds @tamtri/client per connected daemon
  desktop/       Electron-only glue (local IPC transport, titlebar drag region)
```

## Connectivity

The UI never speaks the wire protocol directly. It constructs a `DaemonClient`
from `@tamtri/client` over the right transport:

- desktop: local IPC bridge to the Electron main process (which owns the socket)
- web: direct localhost WebSocket to the daemon
- mobile: relay E2EE channel after QR/URL pairing

Deps (Expo, react-native, expo-router, unistyles, zustand, react-query, etc.)
are added in the app build-out step.
