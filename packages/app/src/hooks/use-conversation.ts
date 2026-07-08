import { useCallback, useEffect, useState } from "react";
import { method, type ConversationDto } from "@tamtri/protocol";

import { parseTranscript, type TranscriptMessage } from "@/lib/transcript";
import { useDaemon } from "@/runtime/daemon-provider";

export function useConversation(conversationId: string | undefined) {
  const { client, subscribe } = useDaemon();
  const [conversation, setConversation] = useState<ConversationDto | null>(null);
  const [messages, setMessages] = useState<TranscriptMessage[]>([]);
  const [loading, setLoading] = useState(Boolean(conversationId));
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!conversationId) return;
    try {
      const dto = await client.request<ConversationDto>(method.CONVERSATION_LOAD, {
        id: conversationId,
      });
      setConversation(dto);
      setMessages(parseTranscript(dto.transcript_json));
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client, conversationId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!conversationId) return;
    return subscribe((event) => {
      if (event.conversation_id !== conversationId) return;
      if (
        event.kind === "message_committed" ||
        event.kind === "text_delta" ||
        event.kind === "turn_ended" ||
        event.kind === "turn_started"
      ) {
        void refresh();
      }
    });
  }, [conversationId, refresh, subscribe]);

  const sendMessage = useCallback(
    async (text: string) => {
      if (!conversationId || !text.trim()) return;
      setSending(true);
      try {
        await client.request(method.CONVERSATION_SEND_MESSAGE, {
          conversation_id: conversationId,
          text: text.trim(),
        });
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setSending(false);
      }
    },
    [client, conversationId, refresh],
  );

  return { conversation, messages, loading, sending, error, refresh, sendMessage };
}
