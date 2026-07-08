export interface PendingElicitation {
  requestId: string;
  serverId: string;
  mode: "form" | "url" | string;
  message: string;
  schema?: unknown;
  url?: string;
  originToolCallId?: string;
}

export function parseElicitationRequested(payloadJson: string): PendingElicitation | null {
  try {
    const raw = JSON.parse(payloadJson) as Record<string, unknown>;
    if (raw.type !== "elicitation_requested") return null;
    return {
      requestId: String(raw.request_id ?? ""),
      serverId: String(raw.server_id ?? "downstream"),
      mode: String(raw.mode ?? "form"),
      message: String(raw.message ?? "Additional input required"),
      schema: raw.schema,
      url: raw.url ? String(raw.url) : undefined,
      originToolCallId: raw.origin_tool_call_id ? String(raw.origin_tool_call_id) : undefined,
    };
  } catch {
    return null;
  }
}
