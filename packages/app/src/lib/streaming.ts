import type { ContentBlock, TranscriptMessage } from "@/lib/transcript";

/** Mirrors the in-flight portion of core's TurnReducer for live UI updates. */
export interface LiveTurnState {
  textBuffer: string;
  thoughtBuffer: string;
  blocks: ContentBlock[];
}

export function createLiveTurn(): LiveTurnState {
  return { textBuffer: "", thoughtBuffer: "", blocks: [] };
}

function flushText(state: LiveTurnState): LiveTurnState {
  if (!state.textBuffer) return state;
  return {
    ...state,
    textBuffer: "",
    blocks: [...state.blocks, { type: "text", text: state.textBuffer }],
  };
}

function flushThought(state: LiveTurnState): LiveTurnState {
  if (!state.thoughtBuffer) return state;
  return {
    ...state,
    thoughtBuffer: "",
    blocks: [...state.blocks, { type: "thinking", text: state.thoughtBuffer }],
  };
}

function flushDeltas(state: LiveTurnState): LiveTurnState {
  return flushThought(flushText(state));
}

function toolOutput(content: unknown[], status: string): unknown {
  if (content.length === 1 && typeof content[0] === "object" && content[0] !== null) {
    const item = content[0] as Record<string, unknown>;
    if (item.type === "text" && typeof item.text === "string") return item.text;
    if (item.type === "json") return item.value;
  }
  return { status, content };
}

/** Apply a harness UiEvent payload to the live turn state. */
export function applyHarnessPayload(state: LiveTurnState, payloadJson: string): LiveTurnState {
  let event: Record<string, unknown>;
  try {
    event = JSON.parse(payloadJson) as Record<string, unknown>;
  } catch {
    return state;
  }

  const kind = event.type as string | undefined;
  switch (kind) {
    case "text_delta": {
      const next = flushThought(state);
      return { ...next, textBuffer: next.textBuffer + String(event.text ?? "") };
    }
    case "thought_delta": {
      const next = flushText(state);
      return { ...next, thoughtBuffer: next.thoughtBuffer + String(event.text ?? "") };
    }
    case "tool_call_started": {
      const flushed = flushDeltas(state);
      return {
        ...flushed,
        blocks: [
          ...flushed.blocks,
          {
            type: "tool_call",
            id: String(event.id ?? ""),
            name: String(event.name ?? ""),
            input: event.input,
          },
        ],
      };
    }
    case "tool_call_progress": {
      const status = String(event.status ?? "");
      if (status !== "completed" && status !== "failed") return flushDeltas(state);
      const content = Array.isArray(event.content) ? event.content : [];
      const flushed = flushDeltas(state);
      return {
        ...flushed,
        blocks: [
          ...flushed.blocks,
          {
            type: "tool_result",
            call_id: String(event.id ?? ""),
            output: toolOutput(content, status),
          },
        ],
      };
    }
    case "error": {
      const flushed = flushDeltas(state);
      return {
        ...flushed,
        blocks: [
          ...flushed.blocks,
          { type: "text", text: `Error: ${String(event.message ?? "unknown")}` },
        ],
      };
    }
    default:
      return state;
  }
}

export function liveTurnToMessage(
  state: LiveTurnState,
  harnessId?: string,
): TranscriptMessage | null {
  let merged = flushDeltas(state);
  if (!merged.textBuffer && !merged.thoughtBuffer && merged.blocks.length === 0) {
    return null;
  }
  merged = flushDeltas(merged);
  return {
    id: "live",
    role: "assistant",
    harness_id: harnessId,
    content: merged.blocks,
    created_at: new Date().toISOString(),
  };
}

export function parseCommittedMessage(payloadJson: string): TranscriptMessage | null {
  try {
    const raw = JSON.parse(payloadJson) as Record<string, unknown>;
    if (raw.role !== "assistant" && raw.role !== "user") return null;
    const content = Array.isArray(raw.content) ? raw.content : [];
    return {
      id: String(raw.id ?? ""),
      role: raw.role as TranscriptMessage["role"],
      harness_id: raw.harness_id ? String(raw.harness_id) : undefined,
      content: content.map((block) => {
        if (typeof block === "object" && block && "type" in block) {
          const b = block as Record<string, unknown>;
          switch (b.type) {
            case "text":
              return { type: "text" as const, text: String(b.text ?? "") };
            case "thinking":
              return { type: "thinking" as const, text: String(b.text ?? "") };
            case "tool_call":
              return {
                type: "tool_call" as const,
                id: String(b.id ?? ""),
                name: String(b.name ?? ""),
                input: b.input,
              };
            case "tool_result":
              return {
                type: "tool_result" as const,
                call_id: String(b.call_id ?? ""),
                output: b.output,
              };
            default:
              return { type: "unknown" as const, raw: block };
          }
        }
        return { type: "unknown" as const, raw: block };
      }),
      created_at: String(raw.created_at ?? new Date().toISOString()),
    };
  } catch {
    return null;
  }
}
