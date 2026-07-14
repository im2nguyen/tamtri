import type { UiEvent } from "@tamtri/protocol";
import type {
  DynamicToolUIPart,
  TamtriDataPart,
  TamtriUIMessage,
  TamtriUIMessagePart,
  UIMessageChunk,
} from "@tamtri/protocol";

import { parseElicitationRequested } from "@/lib/elicitation";
import { parsePermissionRequested } from "@/lib/permissions";

const ORCHESTRATION_KINDS = new Set([
  "orchestration_started",
  "orchestration_step_started",
  "orchestration_forked",
  "orchestration_branch_completed",
  "orchestration_finished",
]);

let nextPartId = 0;

function freshPartId(prefix: string): string {
  nextPartId += 1;
  return `${prefix}-${nextPartId}`;
}

function toolOutput(content: unknown[], status: string): unknown {
  if (content.length === 1 && typeof content[0] === "object" && content[0] !== null) {
    const item = content[0] as Record<string, unknown>;
    if (item.type === "text" && typeof item.text === "string") return item.text;
    if (item.type === "json") return item.value;
  }
  return { status, content };
}

/**
 * Map a daemon UiEvent to incremental UIMessage chunks for live reconciliation.
 * Returns null when the event does not affect the in-flight assistant message.
 */
export function uiMessageChunksFromUiEvent(event: UiEvent): UIMessageChunk[] | null {
  switch (event.kind) {
    case "text_delta": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      const delta = String(payload.text ?? "");
      if (!delta) return null;
      return [{ type: "text-delta", id: "live-text", delta }];
    }
    case "thought_delta": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      const delta = String(payload.text ?? "");
      if (!delta) return null;
      return [{ type: "reasoning-delta", id: "live-reasoning", delta }];
    }
    case "tool_call_started": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      const toolCallId = String(payload.id ?? freshPartId("tool"));
      const toolName = String(payload.name ?? "unknown");
      return [
        { type: "tool-input-start", toolCallId, toolName },
        { type: "tool-input-end", toolCallId, input: payload.input },
      ];
    }
    case "tool_call_progress": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      const status = String(payload.status ?? "");
      if (status !== "completed" && status !== "failed") return null;
      const toolCallId = String(payload.id ?? "");
      const content = Array.isArray(payload.content) ? payload.content : [];
      if (status === "failed") {
        return [{ type: "tool-output-error", toolCallId, errorText: "Tool call failed" }];
      }
      return [{ type: "tool-output-available", toolCallId, output: toolOutput(content, status) }];
    }
    case "error": {
      const payload = parsePayload(event.payload_json);
      const message = payload ? String(payload.message ?? "unknown") : "unknown";
      return [{ type: "error-text", message }];
    }
    case "permission_requested": {
      const parsed = parsePermissionRequested(event.payload_json);
      if (!parsed) return null;
      return [
        {
          type: "data-part",
          part: {
            type: "data-tamtri-permission",
            id: parsed.requestId,
            data: {
              request_id: parsed.requestId,
              action: parsed.action,
              detail: parsed.detail,
              options: parsed.options,
              harness_display_name: parsed.harnessDisplayName,
            },
          } satisfies TamtriDataPart,
        },
      ];
    }
    case "permission_resolved": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      return [
        {
          type: "data-part",
          part: {
            type: "data-tamtri-permission",
            id: String(payload.request_id ?? "permission"),
            data: {
              request_id: String(payload.request_id ?? ""),
              action: "resolved",
              detail: payload,
              resolved: true,
              option_id: payload.option_id ? String(payload.option_id) : undefined,
            },
          } satisfies TamtriDataPart,
        },
      ];
    }
    case "elicitation_requested": {
      const parsed = parseElicitationRequested(event.payload_json);
      if (!parsed) return null;
      return [
        {
          type: "data-part",
          part: {
            type: "data-tamtri-elicitation",
            id: parsed.requestId,
            data: {
              request_id: parsed.requestId,
              phase: "request",
              message: parsed.message,
              mode: parsed.mode,
              server_id: parsed.serverId,
              origin_tool_call_id: parsed.originToolCallId,
              schema: parsed.schema,
              url: parsed.url,
            },
          } satisfies TamtriDataPart,
        },
      ];
    }
    case "elicitation_resolved": {
      const payload = parsePayload(event.payload_json);
      if (!payload) return null;
      return [
        {
          type: "data-part",
          part: {
            type: "data-tamtri-elicitation",
            id: `${String(payload.request_id ?? "elicitation")}-response`,
            data: {
              request_id: String(payload.request_id ?? ""),
              phase: "response",
              action: payload.action ? String(payload.action) : undefined,
              data: payload.data,
            },
          } satisfies TamtriDataPart,
        },
      ];
    }
    default:
      if (ORCHESTRATION_KINDS.has(event.kind)) {
        const payload = parsePayload(event.payload_json);
        return [
          {
            type: "data-part",
            part: {
              type: "data-tamtri-orchestration",
              id: payload?.run_id ? String(payload.run_id) : event.kind,
              data: {
                kind: event.kind,
                run_id: payload?.run_id ? String(payload.run_id) : undefined,
                payload: payload ?? event.payload_json,
              },
            } satisfies TamtriDataPart,
          },
        ];
      }
      return null;
  }
}

function parsePayload(payloadJson: string): Record<string, unknown> | null {
  try {
    return JSON.parse(payloadJson) as Record<string, unknown>;
  } catch {
    return null;
  }
}

export interface LiveUiProjectionState {
  message: TamtriUIMessage;
  activeTextPartId: string | null;
  activeReasoningPartId: string | null;
  openToolCalls: Map<string, number>;
}

export function createLiveUiProjection(harnessId?: string): LiveUiProjectionState {
  return {
    message: {
      id: "live",
      role: "assistant",
      metadata: harnessId ? { harness_id: harnessId } : undefined,
      parts: [],
    },
    activeTextPartId: null,
    activeReasoningPartId: null,
    openToolCalls: new Map(),
  };
}

function closeTextStream(state: LiveUiProjectionState): LiveUiProjectionState {
  if (!state.activeTextPartId) return state;
  const parts = state.message.parts.map((part) =>
    part.type === "text" && part.state === "streaming"
      ? { ...part, state: "done" as const }
      : part,
  );
  return {
    ...state,
    message: { ...state.message, parts },
    activeTextPartId: null,
  };
}

function closeReasoningStream(state: LiveUiProjectionState): LiveUiProjectionState {
  if (!state.activeReasoningPartId) return state;
  const parts = state.message.parts.map((part) =>
    part.type === "reasoning" && part.state === "streaming"
      ? { ...part, state: "done" as const }
      : part,
  );
  return {
    ...state,
    message: { ...state.message, parts },
    activeReasoningPartId: null,
  };
}

function findToolPartIndex(parts: TamtriUIMessagePart[], toolCallId: string): number {
  return parts.findIndex(
    (part) =>
      part.type.startsWith("tool-") &&
      (part as DynamicToolUIPart).toolCallId === toolCallId,
  );
}

function applyChunk(state: LiveUiProjectionState, chunk: UIMessageChunk): LiveUiProjectionState {
  const parts = [...state.message.parts];
  let activeTextPartId = state.activeTextPartId;
  let activeReasoningPartId = state.activeReasoningPartId;
  const openToolCalls = new Map(state.openToolCalls);

  switch (chunk.type) {
    case "text-start": {
      const next = closeReasoningStream({
        ...state,
        message: { ...state.message, parts },
        activeTextPartId,
        activeReasoningPartId,
        openToolCalls,
      });
      parts.splice(0, parts.length, ...next.message.parts);
      activeReasoningPartId = next.activeReasoningPartId;
      if (activeTextPartId === chunk.id) break;
      parts.push({ type: "text", text: "", state: "streaming" });
      activeTextPartId = chunk.id;
      break;
    }
    case "text-delta": {
      if (activeTextPartId !== chunk.id) {
        const next = closeReasoningStream({
          ...state,
          message: { ...state.message, parts },
          activeTextPartId,
          activeReasoningPartId,
          openToolCalls,
        });
        parts.splice(0, parts.length, ...next.message.parts);
        activeReasoningPartId = next.activeReasoningPartId;
        parts.push({ type: "text", text: chunk.delta, state: "streaming" });
        activeTextPartId = chunk.id;
        break;
      }
      const idx = parts.findIndex((part) => part.type === "text" && part.state === "streaming");
      if (idx >= 0) {
        const textPart = parts[idx];
        if (textPart.type === "text") {
          parts[idx] = { ...textPart, text: textPart.text + chunk.delta };
        }
      } else {
        parts.push({ type: "text", text: chunk.delta, state: "streaming" });
      }
      break;
    }
    case "text-end": {
      const idx = parts.findIndex((part) => part.type === "text" && part.state === "streaming");
      if (idx >= 0) {
        const textPart = parts[idx];
        if (textPart.type === "text") {
          parts[idx] = { ...textPart, state: "done" };
        }
      }
      activeTextPartId = null;
      break;
    }
    case "reasoning-start": {
      const next = closeTextStream({
        ...state,
        message: { ...state.message, parts },
        activeTextPartId,
        activeReasoningPartId,
        openToolCalls,
      });
      parts.splice(0, parts.length, ...next.message.parts);
      activeTextPartId = next.activeTextPartId;
      if (activeReasoningPartId === chunk.id) break;
      parts.push({ type: "reasoning", text: "", state: "streaming" });
      activeReasoningPartId = chunk.id;
      break;
    }
    case "reasoning-delta": {
      if (activeReasoningPartId !== chunk.id) {
        const next = closeTextStream({
          ...state,
          message: { ...state.message, parts },
          activeTextPartId,
          activeReasoningPartId,
          openToolCalls,
        });
        parts.splice(0, parts.length, ...next.message.parts);
        activeTextPartId = next.activeTextPartId;
        parts.push({ type: "reasoning", text: chunk.delta, state: "streaming" });
        activeReasoningPartId = chunk.id;
        break;
      }
      const idx = parts.findIndex((part) => part.type === "reasoning" && part.state === "streaming");
      if (idx >= 0) {
        const reasoningPart = parts[idx];
        if (reasoningPart.type === "reasoning") {
          parts[idx] = { ...reasoningPart, text: reasoningPart.text + chunk.delta };
        }
      } else {
        parts.push({ type: "reasoning", text: chunk.delta, state: "streaming" });
      }
      break;
    }
    case "reasoning-end": {
      const idx = parts.findIndex((part) => part.type === "reasoning" && part.state === "streaming");
      if (idx >= 0) {
        const reasoningPart = parts[idx];
        if (reasoningPart.type === "reasoning") {
          parts[idx] = { ...reasoningPart, state: "done" };
        }
      }
      activeReasoningPartId = null;
      break;
    }
    case "tool-input-start": {
      const next = closeTextStream(
        closeReasoningStream({
          ...state,
          message: { ...state.message, parts },
          activeTextPartId,
          activeReasoningPartId,
          openToolCalls,
        }),
      );
      parts.splice(0, parts.length, ...next.message.parts);
      activeTextPartId = next.activeTextPartId;
      activeReasoningPartId = next.activeReasoningPartId;
      const toolPart: DynamicToolUIPart = {
        type: `tool-${chunk.toolName}`,
        toolCallId: chunk.toolCallId,
        state: "input-streaming",
      };
      openToolCalls.set(chunk.toolCallId, parts.length);
      parts.push(toolPart);
      break;
    }
    case "tool-input-end": {
      const idx = openToolCalls.get(chunk.toolCallId) ?? findToolPartIndex(parts, chunk.toolCallId);
      if (idx >= 0) {
        const existing = parts[idx] as DynamicToolUIPart;
        parts[idx] = {
          ...existing,
          state: "input-available",
          input: chunk.input,
        };
        openToolCalls.set(chunk.toolCallId, idx);
      }
      break;
    }
    case "tool-output-available": {
      const idx = openToolCalls.get(chunk.toolCallId) ?? findToolPartIndex(parts, chunk.toolCallId);
      if (idx >= 0) {
        const existing = parts[idx] as DynamicToolUIPart;
        parts[idx] = {
          ...existing,
          state: "output-available",
          output: chunk.output,
        };
        openToolCalls.delete(chunk.toolCallId);
      } else {
        parts.push({
          type: "tool-result",
          toolCallId: chunk.toolCallId,
          state: "output-available",
          output: chunk.output,
        });
      }
      break;
    }
    case "tool-output-error": {
      const idx = openToolCalls.get(chunk.toolCallId) ?? findToolPartIndex(parts, chunk.toolCallId);
      if (idx >= 0) {
        const existing = parts[idx] as DynamicToolUIPart;
        parts[idx] = {
          ...existing,
          state: "output-error",
          errorText: chunk.errorText,
        };
        openToolCalls.delete(chunk.toolCallId);
      }
      break;
    }
    case "data-part":
      parts.push(chunk.part);
      break;
    case "error-text":
      parts.push({ type: "text", text: `Error: ${chunk.message}`, state: "done" });
      break;
    default:
      break;
  }

  return {
    message: { ...state.message, parts },
    activeTextPartId,
    activeReasoningPartId,
    openToolCalls,
  };
}

export function applyUiMessageChunks(
  state: LiveUiProjectionState,
  chunks: UIMessageChunk[],
): LiveUiProjectionState {
  return chunks.reduce(applyChunk, state);
}

/** Finalize open text/reasoning streams when a turn ends without a commit. */
export function finalizeLiveUiProjection(state: LiveUiProjectionState): LiveUiProjectionState {
  let next = state;
  if (next.activeTextPartId) {
    next = applyChunk(next, { type: "text-end", id: next.activeTextPartId });
  }
  if (next.activeReasoningPartId) {
    next = applyChunk(next, { type: "reasoning-end", id: next.activeReasoningPartId });
  }
  return next;
}
