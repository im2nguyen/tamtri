/**
 * AI SDK–compatible UIMessage types for tamtri renderers.
 *
 * Vault truth stays `ContentBlock` in messages.jsonl (Rust). These types are
 * a view-model projection for UI consumption only.
 *
 * ContentBlock → UIMessagePart mapping:
 * - `text`              → `{ type: 'text', text, state: 'done' }`
 * - `thinking`          → `{ type: 'reasoning', text, state: 'done' }`
 * - `tool_call`         → `{ type: 'tool-${name}', toolCallId, state: 'input-available', input }`
 * - `tool_result`       → same tool part with `state: 'output-available'` (merged when adjacent)
 * - `artifact`          → `{ type: 'data-tamtri-artifact', data: … }`
 * - `elicitation_request` / `elicitation_response` → `data-tamtri-elicitation`
 * - `app_resource`      → `{ type: 'data-tamtri-app-resource', data: … }`
 * - `task_ref`          → `{ type: 'data-tamtri-task', data: … }`
 * - permission (live)   → `{ type: 'data-tamtri-permission', data: … }` (UiEvent only today)
 */

/** Custom data carried by `data-tamtri-*` parts. */
export interface TamtriArtifactData {
  path: string;
  mime_type: string;
  size: number;
  sha256?: string;
  inline?: string;
  integrity_failed?: boolean;
}

export interface TamtriElicitationData {
  request_id: string;
  phase: "request" | "response";
  message?: string;
  mode?: string;
  server_id?: string;
  origin_tool_call_id?: string;
  schema?: unknown;
  url?: string;
  action?: string;
  data?: unknown;
}

export interface TamtriPermissionData {
  request_id: string;
  action: string;
  detail: unknown;
  options?: Array<{ id: string; label: string }>;
  harness_display_name?: string;
  resolved?: boolean;
  option_id?: string;
}

export interface TamtriTaskData {
  task_id: string;
  status: string;
  title?: string;
  result_summary?: string;
  origin_tool_call_id?: string;
}

export interface TamtriAppResourceData {
  uri: string;
  template_ref: string;
  state: unknown;
  server_id?: string;
  origin_tool_call_id?: string;
}

export interface TamtriOrchestrationData {
  kind: string;
  run_id?: string;
  payload: unknown;
}

export interface TamtriDataParts {
  "tamtri-artifact": TamtriArtifactData;
  "tamtri-elicitation": TamtriElicitationData;
  "tamtri-permission": TamtriPermissionData;
  "tamtri-task": TamtriTaskData;
  "tamtri-app-resource": TamtriAppResourceData;
  "tamtri-orchestration": TamtriOrchestrationData;
}

export type TamtriDataPartName = keyof TamtriDataParts;

export type TamtriDataPart = {
  [K in TamtriDataPartName]: {
    type: `data-${K}`;
    id?: string;
    data: TamtriDataParts[K];
  };
}[TamtriDataPartName];

export interface TamtriMessageMetadata {
  harness_id?: string;
  created_at?: string;
}

export interface TextUIPart {
  type: "text";
  text: string;
  state?: "streaming" | "done";
}

export interface ReasoningUIPart {
  type: "reasoning";
  text: string;
  state?: "streaming" | "done";
}

/** Tool part keyed by tool name (`tool-${name}`), matching AI SDK conventions. */
export interface DynamicToolUIPart {
  type: `tool-${string}`;
  toolCallId: string;
  state: "input-streaming" | "input-available" | "output-available" | "output-error";
  input?: unknown;
  output?: unknown;
  errorText?: string;
}

export type TamtriUIMessagePart =
  | TextUIPart
  | ReasoningUIPart
  | DynamicToolUIPart
  | TamtriDataPart;

export interface TamtriUIMessage {
  id: string;
  role: "system" | "user" | "assistant";
  metadata?: TamtriMessageMetadata;
  parts: TamtriUIMessagePart[];
}

/** Incremental stream chunks compatible with AI SDK reconciliation patterns. */
export type UIMessageChunk =
  | { type: "text-start"; id: string }
  | { type: "text-delta"; id: string; delta: string }
  | { type: "text-end"; id: string }
  | { type: "reasoning-start"; id: string }
  | { type: "reasoning-delta"; id: string; delta: string }
  | { type: "reasoning-end"; id: string }
  | { type: "tool-input-start"; toolCallId: string; toolName: string }
  | { type: "tool-input-end"; toolCallId: string; input: unknown }
  | { type: "tool-output-available"; toolCallId: string; output: unknown }
  | { type: "tool-output-error"; toolCallId: string; errorText: string }
  | { type: "data-part"; part: TamtriDataPart }
  | { type: "error-text"; message: string };
