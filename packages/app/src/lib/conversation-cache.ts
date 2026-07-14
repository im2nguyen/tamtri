import type { ConversationDto, TamtriUIMessage } from "@tamtri/protocol";

import type { TranscriptMessage } from "@/lib/transcript";

const CACHE_LIMIT = 24;

export interface CachedConversationView {
  conversation: ConversationDto;
  messages: TranscriptMessage[];
  uiMessages: TamtriUIMessage[];
}

const cache = new Map<string, CachedConversationView>();

export function getCachedConversation(conversationId: string): CachedConversationView | undefined {
  return cache.get(conversationId);
}

export function storeCachedConversation(
  conversationId: string,
  view: CachedConversationView,
): void {
  if (cache.size >= CACHE_LIMIT) {
    const oldest = cache.keys().next().value;
    if (oldest) cache.delete(oldest);
  }
  cache.set(conversationId, view);
}

export function patchCachedConversation(
  conversationId: string,
  patch: Partial<Pick<CachedConversationView, "conversation" | "messages" | "uiMessages">>,
): void {
  const existing = cache.get(conversationId);
  if (!existing) return;
  cache.set(conversationId, { ...existing, ...patch });
}

export function clearConversationCache(conversationId?: string): void {
  if (conversationId) {
    cache.delete(conversationId);
    return;
  }
  cache.clear();
}
