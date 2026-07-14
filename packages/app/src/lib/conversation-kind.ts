import type { ConversationSummaryDto } from "@tamtri/protocol";

/** Extended summary fields that may arrive before protocol types are regenerated. */
export type ConversationSummary = ConversationSummaryDto & {
  kind?: string;
};

export function isExampleConversation(conversation: { title?: string; kind?: string }): boolean {
  const kind = conversation.kind?.toLowerCase();
  if (kind === "example") return true;
  return conversation.title?.startsWith("Example:") ?? false;
}
