import type {
  DynamicToolUIPart,
  TamtriDataPart,
  TamtriMessageMetadata,
  TamtriUIMessage,
  TamtriUIMessagePart,
} from "@tamtri/protocol";

import type { ContentBlock, TranscriptMessage } from "@/lib/transcript";

/**
 * Map one vault ContentBlock to zero or more UIMessage parts.
 * Tool results are emitted as standalone output parts; use mergeToolParts for pairing.
 */
export function contentBlockToParts(block: ContentBlock): TamtriUIMessagePart[] {
  switch (block.type) {
    case "text":
      return block.text ? [{ type: "text", text: block.text, state: "done" }] : [];
    case "thinking":
      return block.text ? [{ type: "reasoning", text: block.text, state: "done" }] : [];
    case "tool_call":
      return [
        {
          type: `tool-${block.name}`,
          toolCallId: block.id,
          state: "input-available",
          input: block.input,
        } satisfies DynamicToolUIPart,
      ];
    case "tool_result":
      return [
        {
          type: "tool-result",
          toolCallId: block.call_id,
          state: "output-available",
          output: block.output,
        } satisfies DynamicToolUIPart,
      ];
    case "artifact":
      return [
        {
          type: "data-tamtri-artifact",
          id: block.path,
          data: {
            path: block.path,
            mime_type: block.mime_type,
            size: block.size,
            sha256: block.sha256,
            inline: block.inline,
            integrity_failed: block.integrity_failed,
          },
        } satisfies TamtriDataPart,
      ];
    case "elicitation_request":
      return [
        {
          type: "data-tamtri-elicitation",
          id: block.request_id,
          data: {
            request_id: block.request_id,
            phase: "request",
            message: block.message,
            mode: block.mode,
            server_id: block.server_id,
            origin_tool_call_id: block.origin_tool_call_id,
            schema: block.schema,
            url: block.url,
          },
        } satisfies TamtriDataPart,
      ];
    case "elicitation_response":
      return [
        {
          type: "data-tamtri-elicitation",
          id: `${block.request_id}-response`,
          data: {
            request_id: block.request_id,
            phase: "response",
            action: block.action,
            data: block.data,
          },
        } satisfies TamtriDataPart,
      ];
    case "app_resource":
      return [
        {
          type: "data-tamtri-app-resource",
          id: block.uri,
          data: {
            uri: block.uri,
            template_ref: block.template_ref,
            state: block.state,
            server_id: block.server_id,
            origin_tool_call_id: block.origin_tool_call_id,
          },
        } satisfies TamtriDataPart,
      ];
    case "task_ref":
      return [
        {
          type: "data-tamtri-task",
          id: block.task_id,
          data: {
            task_id: block.task_id,
            status: block.status,
            title: block.title,
            result_summary: block.result_summary,
            origin_tool_call_id: block.origin_tool_call_id,
          },
        } satisfies TamtriDataPart,
      ];
    default:
      return [];
  }
}

/** Merge adjacent tool_call + tool_result blocks into a single tool part. */
export function mergeToolParts(parts: TamtriUIMessagePart[]): TamtriUIMessagePart[] {
  const merged: TamtriUIMessagePart[] = [];
  const pendingCalls = new Map<string, DynamicToolUIPart>();

  for (const part of parts) {
    if (part.type.startsWith("tool-") && part.type !== "tool-result") {
      const toolPart = part as DynamicToolUIPart;
      if (toolPart.state === "input-available" || toolPart.state === "input-streaming") {
        pendingCalls.set(toolPart.toolCallId, toolPart);
        merged.push(toolPart);
        continue;
      }
    }

    if (part.type === "tool-result") {
      const resultPart = part as DynamicToolUIPart;
      const existing = pendingCalls.get(resultPart.toolCallId);
      if (existing) {
        existing.state = "output-available";
        existing.output = resultPart.output;
        pendingCalls.delete(resultPart.toolCallId);
        continue;
      }
      merged.push(resultPart);
      continue;
    }

    merged.push(part);
  }

  return merged;
}

export function contentBlocksToParts(blocks: ContentBlock[]): TamtriUIMessagePart[] {
  const raw = blocks.flatMap((block) => contentBlockToParts(block));
  return mergeToolParts(raw);
}

function mapRole(role: TranscriptMessage["role"]): TamtriUIMessage["role"] {
  if (role === "system") return "system";
  if (role === "user") return "user";
  return "assistant";
}

export function transcriptMessageToUIMessage(message: TranscriptMessage): TamtriUIMessage {
  const metadata: TamtriMessageMetadata = {
    created_at: message.created_at,
  };
  if (message.harness_id) metadata.harness_id = message.harness_id;

  return {
    id: message.id,
    role: mapRole(message.role),
    metadata,
    parts: contentBlocksToParts(message.content),
  };
}

/** Project committed transcript messages to AI SDK–compatible UIMessages. */
export function projectTranscriptToUIMessages(messages: TranscriptMessage[]): TamtriUIMessage[] {
  return messages.map(transcriptMessageToUIMessage);
}
