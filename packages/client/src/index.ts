/**
 * @tamtri/client
 *
 * The universal daemon SDK. Every surface (desktop, web, mobile, cli) talks to
 * the Rust tamtri-daemon through a DaemonClient rather than reimplementing the
 * wire protocol.
 *
 * This is a structural stub. The protocol+client spine step fills in the hello
 * handshake, request/response correlation, event subscription, and binary
 * (base64) framing against the shape defined in @tamtri/protocol.
 */

import { PROTOCOL_VERSION, type ClientType } from "@tamtri/protocol";
import type { DaemonTransport, DaemonTransportFactory } from "./transport.js";

export type { DaemonTransport, DaemonTransportFactory } from "./transport.js";

export interface DaemonClientConfig {
  clientType: ClientType;
  transport: DaemonTransportFactory;
  /** Bearer token from ~/.tamtri (direct localhost) or relay pairing. */
  token?: string;
}

export class DaemonClient {
  readonly protocolVersion = PROTOCOL_VERSION;

  constructor(private readonly config: DaemonClientConfig) {}

  async connect(): Promise<void> {
    throw new Error("DaemonClient.connect not implemented (protocol+client spine step)");
  }

  async request<TResult>(_method: string, _params?: unknown): Promise<TResult> {
    throw new Error("DaemonClient.request not implemented (protocol+client spine step)");
  }

  subscribe(_handler: (event: unknown) => void): () => void {
    throw new Error("DaemonClient.subscribe not implemented (protocol+client spine step)");
  }

  close(): void {
    void this.config;
  }
}

export function createDaemonClient(config: DaemonClientConfig): DaemonClient {
  return new DaemonClient(config);
}

// Referenced so the transport contract is part of the public surface.
export type __DaemonTransportContract = DaemonTransport;
