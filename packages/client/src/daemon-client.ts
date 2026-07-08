/**
 * The universal daemon SDK. Opens a transport, performs the hello handshake,
 * correlates JSON-RPC requests with responses by id, and fans out `event`
 * notifications to subscribers. Transport-agnostic: the same client runs over a
 * localhost WebSocket, an Electron IPC bridge, or a relay-encrypted channel.
 */

import {
  PROTOCOL_VERSION,
  ClientType,
  classifyIncoming,
  jsonRpcRequest,
  method,
  type EventNotification,
  type JsonRpcError,
  type RequestId,
  type ServerInfo,
} from "@tamtri/protocol";
import type { DaemonTransport, DaemonTransportFactory } from "./transport.js";

export interface DaemonClientConfig {
  /** Stable id for this client instance (surfaces in the daemon logs). */
  clientId: string;
  clientType: ClientType;
  appVersion?: string;
  transport: DaemonTransportFactory;
}

export class DaemonClientError extends Error {
  constructor(
    message: string,
    readonly code?: number,
    readonly data?: unknown,
  ) {
    super(message);
    this.name = "DaemonClientError";
  }
}

interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

export class DaemonClient {
  readonly protocolVersion = PROTOCOL_VERSION;

  private transport?: DaemonTransport;
  private serverInfo?: ServerInfo;
  private nextId = 1;
  private readonly pending = new Map<RequestId, Pending>();
  private readonly eventHandlers = new Set<(event: EventNotification) => void>();
  private closed = false;

  constructor(private readonly config: DaemonClientConfig) {}

  /** Open the transport and complete the hello handshake. Returns the daemon's
   * identity + capability flags so the caller can gate features. */
  async connect(): Promise<ServerInfo> {
    if (this.transport) throw new DaemonClientError("already connected");
    const transport = await this.config.transport();
    this.transport = transport;
    transport.onMessage((data) => this.handleMessage(data));
    transport.onClose(() => this.handleClose());

    const info = (await this.request(method.HELLO, {
      client_id: this.config.clientId,
      client_type: this.config.clientType,
      protocol_version: PROTOCOL_VERSION,
      ...(this.config.appVersion ? { app_version: this.config.appVersion } : {}),
    })) as ServerInfo;
    this.serverInfo = info;
    return info;
  }

  /** The ServerInfo captured during connect, if any. */
  get info(): ServerInfo | undefined {
    return this.serverInfo;
  }

  /** Issue a JSON-RPC request and resolve with its typed result. */
  request<TResult = unknown>(methodName: string, params?: unknown): Promise<TResult> {
    if (!this.transport) return Promise.reject(new DaemonClientError("not connected"));
    if (this.closed) return Promise.reject(new DaemonClientError("connection closed"));
    const id = this.nextId++;
    const frame = JSON.stringify(jsonRpcRequest(id, methodName, params));
    return new Promise<TResult>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (value: unknown) => void, reject });
      this.transport?.send(frame);
    });
  }

  /** Subscribe to daemon `event` notifications. Returns an unsubscribe fn. */
  subscribe(handler: (event: EventNotification) => void): () => void {
    this.eventHandlers.add(handler);
    return () => this.eventHandlers.delete(handler);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.transport?.close();
    this.failPending(new DaemonClientError("connection closed"));
  }

  private handleMessage(data: string): void {
    let parsed: unknown;
    try {
      parsed = JSON.parse(data);
    } catch {
      return;
    }
    const message = classifyIncoming(parsed);
    if (!message) return;

    if (message.kind === "response") {
      const { id, result, error } = message.response;
      const pending = this.pending.get(id);
      if (!pending) return;
      this.pending.delete(id);
      if (error) pending.reject(toError(error));
      else pending.resolve(result);
      return;
    }

    if (message.kind === "notification" && message.notification.method === method.EVENT) {
      const event = message.notification.params as EventNotification | undefined;
      if (event) {
        for (const handler of this.eventHandlers) handler(event);
      }
    }
  }

  private handleClose(): void {
    if (this.closed) return;
    this.closed = true;
    this.failPending(new DaemonClientError("connection closed by daemon"));
  }

  private failPending(error: Error): void {
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
  }
}

function toError(error: JsonRpcError): DaemonClientError {
  return new DaemonClientError(error.message, error.code, error.data);
}

export function createDaemonClient(config: DaemonClientConfig): DaemonClient {
  return new DaemonClient(config);
}
