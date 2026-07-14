/**
 * JSON-RPC 2.0 envelope, mirroring core/src/rpc/jsonrpc.rs. Every daemon frame
 * is one of these as a WebSocket text frame:
 *  - client -> daemon: a request (has `id` and `method`)
 *  - daemon -> client: a response (has `id`, exactly one of `result`/`error`)
 *  - daemon -> client: a notification (has `method`, no `id`) — e.g. `event`
 */

export type RequestId = number | string;

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: RequestId;
  method: string;
  params?: unknown;
}

export interface JsonRpcNotification {
  jsonrpc: "2.0";
  method: string;
  params?: unknown;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: RequestId;
  result?: unknown;
  error?: JsonRpcError;
}

/** JSON-RPC error code the daemon returns for an unknown method. */
export const METHOD_NOT_FOUND = -32601;

export function jsonRpcRequest(id: RequestId, method: string, params?: unknown): JsonRpcRequest {
  return params === undefined
    ? { jsonrpc: "2.0", id, method }
    : { jsonrpc: "2.0", id, method, params };
}

/** Classify a parsed inbound frame so the client can route it. */
export type IncomingMessage =
  | { kind: "response"; response: JsonRpcResponse }
  | { kind: "notification"; notification: JsonRpcNotification }
  | { kind: "request"; request: JsonRpcRequest };

export function classifyIncoming(value: unknown): IncomingMessage | null {
  if (typeof value !== "object" || value === null) return null;
  const record = value as Record<string, unknown>;
  if (record.jsonrpc !== "2.0") return null;
  const hasMethod = typeof record.method === "string";
  const hasId = record.id !== undefined;
  if (hasMethod && hasId) return { kind: "request", request: record as unknown as JsonRpcRequest };
  if (hasMethod) return { kind: "notification", notification: record as unknown as JsonRpcNotification };
  if (hasId) return { kind: "response", response: record as unknown as JsonRpcResponse };
  return null;
}
