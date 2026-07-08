/**
 * The contract exposed to the renderer as `window.tamtri`. Shared by the preload
 * (which implements it over Electron IPC) and the renderer (which consumes it).
 *
 * The renderer never opens a socket or sees the daemon token: the main process
 * owns the daemon connection and bridges frames over IPC. This keeps credentials
 * in the shell, per the tamtri security rules.
 */

export interface TamtriTransportBridge {
  /** Ensure the main process has an open connection to the daemon. */
  open(): Promise<void>;
  /** Send one text frame to the daemon. */
  send(data: string): void;
  /** Register a handler for daemon -> renderer frames. Returns an unsubscribe. */
  onMessage(handler: (data: string) => void): () => void;
  /** Register a handler for connection close. Returns an unsubscribe. */
  onClose(handler: () => void): () => void;
  /** Close the daemon connection. */
  close(): void;
}

export interface TamtriBridge {
  transport: TamtriTransportBridge;
}

declare global {
  interface Window {
    tamtri: TamtriBridge;
  }
}

export const IPC = {
  transportOpen: "tamtri:transport:open",
  transportSend: "tamtri:transport:send",
  transportClose: "tamtri:transport:close",
  transportMessage: "tamtri:transport:message",
  transportClosed: "tamtri:transport:closed",
} as const;
