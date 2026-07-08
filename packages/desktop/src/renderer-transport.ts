/**
 * Renderer-side DaemonTransport that proxies through the `window.tamtri` bridge
 * (Electron IPC) instead of opening a socket directly. The DaemonClient in the
 * renderer uses this factory; everything above the transport is identical to the
 * web and mobile surfaces.
 */

import type { DaemonTransport, DaemonTransportFactory } from "@tamtri/client";

export function createDesktopTransport(): DaemonTransportFactory {
  return async (): Promise<DaemonTransport> => {
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
