import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { method, type ConversationSummaryDto } from "@tamtri/protocol";

import { subscribeConversationListInvalidation } from "@/hooks/conversation-list-invalidation";
import { useDaemon } from "@/runtime/daemon-provider";

interface ConversationListValue {
  conversations: ConversationSummaryDto[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

const ConversationListContext = createContext<ConversationListValue | null>(null);

export function ConversationListProvider({ children }: { children: ReactNode }) {
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
      if (event.kind === "message_committed") {
        void refresh();
      }
    });
  }, [refresh, subscribe]);

  const value = useMemo(
    () => ({ conversations, loading, error, refresh }),
    [conversations, loading, error, refresh],
  );

  return (
    <ConversationListContext.Provider value={value}>{children}</ConversationListContext.Provider>
  );
}

export function useConversationList(): ConversationListValue {
  const value = useContext(ConversationListContext);
  if (!value) {
    throw new Error("useConversationList must be used within ConversationListProvider");
  }
  return value;
}
