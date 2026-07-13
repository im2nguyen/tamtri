import { useCallback, useEffect, useState } from "react";
import { method, type ConversationSummaryDto } from "@tamtri/protocol";

import { subscribeConversationListInvalidation } from "@/hooks/conversation-list-invalidation";
import { useDaemon } from "@/runtime/daemon-provider";

export function useConversationList() {
  const { client, subscribe } = useDaemon();
  const [conversations, setConversations] = useState<ConversationSummaryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const rows = await client.request<ConversationSummaryDto[]>(method.CONVERSATION_LIST);
      setConversations(rows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    return subscribeConversationListInvalidation(() => {
      void refresh();
    });
  }, [refresh]);

  useEffect(() => {
    return subscribe((event) => {
      if (
        event.kind === "message_committed" ||
        event.kind === "turn_started" ||
        event.kind === "turn_ended"
      ) {
        void refresh();
      }
    });
  }, [refresh, subscribe]);

  return { conversations, loading, error, refresh };
}
