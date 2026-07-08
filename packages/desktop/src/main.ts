/**
 * Electron main-process entry (structural stub).
 *
 * Build-out responsibilities:
 *  - create the BrowserWindow (custom titlebar / vibrancy for the polish bar)
 *  - start the daemon via DaemonManager, then load the @tamtri/app bundle
 *  - expose a local-transport IPC bridge so the renderer's DaemonClient can
 *    reach the daemon without opening a socket from browser context
 *  - menu, auto-update, file dialogs, notifications
 */

import type { DaemonManager } from "./daemon-manager.js";

export function bootstrap(_daemon: DaemonManager): void {
  throw new Error("desktop main not implemented (Electron build-out step)");
}
