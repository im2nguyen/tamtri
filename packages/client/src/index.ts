/**
 * @tamtri/client
 *
 * The universal daemon SDK. Every surface (desktop, web, mobile, cli) talks to
 * the Rust tamtri-daemon through a DaemonClient rather than reimplementing the
 * wire protocol.
 */

export type { DaemonTransport, DaemonTransportFactory } from "./transport.js";
export {
  webSocketTransport,
  type WebSocketTransportOptions,
} from "./websocket-transport.js";
export {
  relayTransport,
  type RelayTransportOptions,
} from "./relay-transport.js";
export {
  DaemonClient,
  DaemonClientError,
  createDaemonClient,
  type DaemonClientConfig,
} from "./daemon-client.js";
