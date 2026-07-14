/**
 * Transport abstraction. A DaemonClient runs over any transport that can carry
 * text frames both ways: a direct localhost WebSocket, an Electron local-IPC
 * bridge, or a relay-encrypted channel. Surfaces pick the transport; the client
 * logic (correlation, subscriptions) is transport-agnostic.
 */

export interface DaemonTransport {
  send(data: string): void;
  close(): void;
  onMessage(handler: (data: string) => void): void;
  onClose(handler: () => void): void;
}

export type DaemonTransportFactory = () => Promise<DaemonTransport>;
