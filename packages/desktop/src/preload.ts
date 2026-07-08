/**
 * Electron preload. Exposes the minimal, typed `window.tamtri` bridge to the
 * renderer over IPC. No Node globals leak to the page; the renderer can only do
 * what this surface allows (open/close the daemon transport and pass frames).
 */

import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron";

import { IPC, type TamtriBridge } from "./bridge.js";

const bridge: TamtriBridge = {
  transport: {
    open: () => ipcRenderer.invoke(IPC.transportOpen),
    send: (data) => ipcRenderer.send(IPC.transportSend, data),
    onMessage: (handler) => {
      const listener = (_event: IpcRendererEvent, data: string) => handler(data);
      ipcRenderer.on(IPC.transportMessage, listener);
      return () => ipcRenderer.off(IPC.transportMessage, listener);
    },
    onClose: (handler) => {
      const listener = () => handler();
      ipcRenderer.on(IPC.transportClosed, listener);
      return () => ipcRenderer.off(IPC.transportClosed, listener);
    },
    close: () => ipcRenderer.send(IPC.transportClose),
  },
};

contextBridge.exposeInMainWorld("tamtri", bridge);
