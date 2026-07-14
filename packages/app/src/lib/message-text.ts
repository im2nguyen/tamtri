import type { TamtriUIMessage, TamtriUIMessagePart } from "@tamtri/protocol";

function partToCopyText(part: TamtriUIMessagePart): string {
  switch (part.type) {
    case "text":
    case "reasoning":
      return part.text;
    default:
      if (part.type.startsWith("tool-")) {
        const toolPart = part as {
          type: string;
          input?: unknown;
          output?: unknown;
          errorText?: string;
        };
        const toolName = toolPart.type.startsWith("tool-") ? toolPart.type.slice(5) : "tool";
        const chunks: string[] = [`[${toolName}]`];
        if (toolPart.input !== undefined) {
          chunks.push(
            typeof toolPart.input === "string"
              ? toolPart.input
              : JSON.stringify(toolPart.input, null, 2),
          );
        }
        if (toolPart.errorText) {
          chunks.push(toolPart.errorText);
        } else if (toolPart.output !== undefined) {
          chunks.push(
            typeof toolPart.output === "string"
              ? toolPart.output
              : JSON.stringify(toolPart.output, null, 2),
          );
        }
        return chunks.join("\n");
      }
      return "";
  }
}

/** Plain text suitable for clipboard copy from a message's renderable parts. */
export function messageCopyText(message: TamtriUIMessage): string {
  return message.parts
    .map(partToCopyText)
    .filter((chunk) => chunk.trim().length > 0)
    .join("\n\n")
    .trim();
}
