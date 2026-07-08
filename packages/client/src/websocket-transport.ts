/**
 * WebSocket transport for a direct daemon connection (localhost web, or any host
 * reachable over ws/wss). The daemon authenticates via a `token` query param
 * (see daemon/src/server.rs), so the token is baked into the URL here.
 *
 * Uses the platform's global `WebSocket` (browsers, React Native, Node >= 22) by
 * default; a custom implementation can be injected for tests or older runtimes.
 */

import type { DaemonTransport, DaemonTransportFactory } from "./transport.js";

interface WebSocketLike {
  send(data: string): void;
  close(): void;
  addEventListener(type: string, listener: (event: unknown) => void): void;
}

type WebSocketCtor = new (url: string) => WebSocketLike;

export interface WebSocketTransportOptions {
  /** Base URL, e.g. `ws://127.0.0.1:8377/ws`. */
  url: string;
  /** Bearer token from ~/.tamtri (daemon.token) or relay pairing. */
  token: string;
  /** Override the WebSocket constructor (tests, non-global runtimes). */
  webSocketImpl?: WebSocketCtor;
}

function withToken(url: string, token: string): string {
  const separator = url.includes("?") ? "&" : "?";
  return `${url}${separator}token=${encodeURIComponent(token)}`;
}

function resolveImpl(override?: WebSocketCtor): WebSocketCtor {
  if (override) return override;
  const global = (globalThis as { WebSocket?: WebSocketCtor }).WebSocket;
  if (!global) {
    throw new Error("No global WebSocket available; pass webSocketImpl explicitly");
  }
  return global;
}

export function webSocketTransport(options: WebSocketTransportOptions): DaemonTransportFactory {
  return () =>
    new Promise<DaemonTransport>((resolve, reject) => {
      const Impl = resolveImpl(options.webSocketImpl);
      const socket = new Impl(withToken(options.url, options.token));
      let settled = false;

      socket.addEventListener("open", () => {
        settled = true;
        resolve({
          send: (data) => socket.send(data),
          close: () => socket.close(),
          onMessage: (handler) => {
            socket.addEventListener("message", (event) => {
              const data = (event as { data?: unknown }).data;
              if (typeof data === "string") handler(data);
            });
          },
          onClose: (handler) => {
            socket.addEventListener("close", () => handler());
          },
        });
      });

      socket.addEventListener("error", () => {
        if (!settled) reject(new Error(`WebSocket connection to ${options.url} failed`));
      });
    });
}
