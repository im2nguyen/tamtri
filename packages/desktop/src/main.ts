/**
 * Electron main-process entry. Owns the app lifecycle, the daemon subprocess,
 * and the window. The renderer reaches the daemon only through an IPC-bridged
 * transport wired here, so the daemon token stays in the shell.
 *
 * The renderer currently loads the bootstrap splash (a live proof of the
 * desktop -> daemon path). When the Expo web bundle lands, point `loadRenderer`
 * at it (dev: the Metro dev-server URL via TAMTRI_DEV_URL; packaged: the
 * exported static files) with no change to the daemon/IPC wiring.
 */

import { join } from "node:path";

import { webSocketTransport, type DaemonTransport } from "@tamtri/client";
import { app, BrowserWindow, dialog, ipcMain, shell, type IpcMainInvokeEvent } from "electron";

import { DaemonManager, type DaemonEndpoint } from "./daemon-manager.js";
import { IPC } from "./bridge.js";

let manager: DaemonManager | undefined;
let endpoint: DaemonEndpoint | undefined;
let transport: DaemonTransport | undefined;
let mainWindow: BrowserWindow | undefined;

function resolveDaemonBinary(): string {
  if (process.env.TAMTRI_DAEMON_BIN) return process.env.TAMTRI_DAEMON_BIN;
  if (app.isPackaged) return join(process.resourcesPath, "tamtri-daemon");
  // dist/main.js -> repo root is three levels up (packages/desktop/dist).
  return join(__dirname, "..", "..", "..", "target", "debug", "tamtri-daemon");
}

function send(channel: string, ...args: unknown[]): void {
  mainWindow?.webContents.send(channel, ...args);
}

async function ensureTransport(): Promise<void> {
  if (transport || !endpoint) return;
  const factory = webSocketTransport({
    url: `ws://127.0.0.1:${endpoint.port}/ws`,
    token: endpoint.token,
  });
  const opened = await factory();
  opened.onMessage((data) => send(IPC.transportMessage, data));
  opened.onClose(() => {
    transport = undefined;
    send(IPC.transportClosed);
  });
  transport = opened;
}

function registerIpc(): void {
  ipcMain.handle(IPC.transportOpen, async (_event: IpcMainInvokeEvent) => {
    await ensureTransport();
  });
  ipcMain.on(IPC.transportSend, (_event, data: string) => {
    transport?.send(data);
  });
  ipcMain.on(IPC.transportClose, () => {
    transport?.close();
    transport = undefined;
  });
  ipcMain.handle(IPC.shellPickOpenFile, async (_event, options?: { title?: string; filters?: { name: string; extensions: string[] }[] }) => {
    const result = await dialog.showOpenDialog({
      title: options?.title,
      properties: ["openFile", "openDirectory"],
      filters: options?.filters,
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    return result.filePaths[0] ?? null;
  });
  ipcMain.handle(
    IPC.shellPickSaveFile,
    async (_event, options?: { title?: string; defaultPath?: string; filters?: { name: string; extensions: string[] }[] }) => {
      const result = await dialog.showSaveDialog({
        title: options?.title,
        defaultPath: options?.defaultPath,
        filters: options?.filters,
      });
      if (result.canceled || !result.filePath) return null;
      return result.filePath;
    },
  );
  ipcMain.handle(IPC.shellShowItemInFolder, async (_event, filePath: string) => {
    shell.showItemInFolder(filePath);
  });
}

async function loadRenderer(window: BrowserWindow): Promise<void> {
  const devUrl = process.env.TAMTRI_DEV_URL;
  const useDevServer = process.env.TAMTRI_USE_DEV_SERVER === "1" || Boolean(devUrl);
  if (useDevServer) {
    await window.loadURL(devUrl ?? "http://localhost:8081");
    return;
  }

  const { existsSync } = await import("node:fs");
  const appIndex = join(__dirname, "renderer", "app", "index.html");
  if (existsSync(appIndex)) {
    await window.loadFile(appIndex);
    return;
  }

  await window.loadFile(join(__dirname, "renderer", "index.html"));
}

async function createWindow(): Promise<void> {
  mainWindow = new BrowserWindow({
    width: 1100,
    height: 760,
    minWidth: 720,
    minHeight: 480,
    show: false,
    backgroundColor: "#181B1A",
    titleBarStyle: "hiddenInset",
    webPreferences: {
      preload: join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  mainWindow.once("ready-to-show", () => mainWindow?.show());
  mainWindow.on("closed", () => {
    mainWindow = undefined;
  });
  await loadRenderer(mainWindow);
}

async function bootstrap(): Promise<void> {
  manager = new DaemonManager({ binaryPath: resolveDaemonBinary() });
  manager.on("exit", () => {
    transport = undefined;
    endpoint = undefined;
    send(IPC.transportClosed);
  });
  endpoint = await manager.start();
  registerIpc();
  await createWindow();
}

app.whenReady().then(bootstrap).catch((error) => {
  console.error("failed to start tamtri desktop:", error);
  app.quit();
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) void createWindow();
});

app.on("window-all-closed", () => {
  void (async () => {
    transport?.close();
    await manager?.stop();
    if (process.platform !== "darwin") app.quit();
  })();
});

app.on("before-quit", () => {
  transport?.close();
  void manager?.stop();
});
