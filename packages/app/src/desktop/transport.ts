import type { DaemonTransport, DaemonTransportFactory } from "@tamtri/client";

export function createDesktopTransport(): DaemonTransportFactory {
  return async (): Promise<DaemonTransport> => {
    if (typeof window === "undefined" || !window.tamtri?.transport) {
      throw new Error("desktop transport unavailable");
    }
    const bridge = window.tamtri.transport;
    await bridge.open();
    return {
      send: (data) => bridge.send(data),
      close: () => bridge.close(),
      onMessage: (handler) => bridge.onMessage(handler),
      onClose: (handler) => bridge.onClose(handler),
    };
  };
}

declare global {
  interface Window {
    tamtri?: {
      transport: {
        open(): Promise<void>;
        send(data: string): void;
        close(): void;
        onMessage(handler: (data: string) => void): () => void;
        onClose(handler: () => void): () => void;
      };
    };
  }
}

export {};
