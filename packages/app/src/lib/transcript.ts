export interface TranscriptMessage {
  id: string;
  role: "user" | "assistant" | "tool" | "system";
  harness_id?: string;
  content: ContentBlock[];
  created_at: string;
}

export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "thinking"; text: string }
  | { type: "tool_call"; id: string; name: string; input: unknown }
  | { type: "tool_result"; call_id: string; output: unknown }
  | {
      type: "artifact";
      path: string;
      mime_type: string;
      size: number;
      sha256?: string;
      inline?: string;
      integrity_failed?: boolean;
    }
  | {
      type: "app_resource";
      uri: string;
      template_ref: string;
      state: unknown;
      server_id?: string;
      origin_tool_call_id?: string;
    }
  | {
      type: "elicitation_request";
      request_id: string;
      message: string;
      mode: string;
      server_id?: string;
      origin_tool_call_id?: string;
      schema?: unknown;
      url?: string;
    }
  | {
      type: "elicitation_response";
      request_id: string;
      action: string;
      data?: unknown;
    }
  | {
      type: "task_ref";
      task_id: string;
      status: string;
      title?: string;
      result_summary?: string;
      origin_tool_call_id?: string;
    }
  | { type: "unknown"; raw: unknown };

export function parseTranscript(json: string): TranscriptMessage[] {
  try {
    const parsed = JSON.parse(json) as TranscriptMessage[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function normalizeBlock(raw: Record<string, unknown>): ContentBlock {
  const type = raw.type as string;
  switch (type) {
    case "text":
      return { type: "text", text: String(raw.text ?? "") };
    case "thinking":
      return { type: "thinking", text: String(raw.text ?? "") };
    case "tool_call":
      return {
        type: "tool_call",
        id: String(raw.id ?? ""),
        name: String(raw.name ?? ""),
        input: raw.input,
      };
    case "tool_result":
      return {
        type: "tool_result",
        call_id: String(raw.call_id ?? ""),
        output: raw.output,
      };
    case "artifact":
      return {
        type: "artifact",
        path: String(raw.path ?? ""),
        mime_type: String(raw.mime_type ?? ""),
        size: Number(raw.size ?? 0),
        sha256: raw.sha256 ? String(raw.sha256) : undefined,
        inline: raw.inline ? String(raw.inline) : undefined,
        integrity_failed: raw.integrity_failed === true,
      };
    case "elicitation_request":
      return {
        type: "elicitation_request",
        request_id: String(raw.request_id ?? ""),
        message: String(raw.message ?? ""),
        mode: String(raw.mode ?? ""),
        server_id: raw.server_id ? String(raw.server_id) : undefined,
        origin_tool_call_id: raw.origin_tool_call_id ? String(raw.origin_tool_call_id) : undefined,
        schema: raw.schema,
        url: raw.url ? String(raw.url) : undefined,
      };
    case "elicitation_response":
      return {
        type: "elicitation_response",
        request_id: String(raw.request_id ?? ""),
        action: String(raw.action ?? ""),
        data: raw.data,
      };
    case "app_resource":
      return {
        type: "app_resource",
        uri: String(raw.uri ?? ""),
        template_ref: String(raw.template_ref ?? ""),
        state: raw.state,
        server_id: raw.server_id ? String(raw.server_id) : undefined,
        origin_tool_call_id: raw.origin_tool_call_id ? String(raw.origin_tool_call_id) : undefined,
      };
    case "task_ref":
      return {
        type: "task_ref",
        task_id: String(raw.task_id ?? ""),
        status: String(raw.status ?? ""),
        title: raw.title ? String(raw.title) : undefined,
        result_summary: raw.result_summary ? String(raw.result_summary) : undefined,
        origin_tool_call_id: raw.origin_tool_call_id ? String(raw.origin_tool_call_id) : undefined,
      };
    default:
      return { type: "unknown", raw };
  }
}
