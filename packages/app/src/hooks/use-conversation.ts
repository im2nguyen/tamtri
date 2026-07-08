import { useCallback, useEffect, useMemo, useState } from "react";
import { method, type ConversationDto } from "@tamtri/protocol";

import { parsePermissionRequested, type PendingPermission } from "@/lib/permissions";
import {
  applyHarnessPayload,
  createLiveTurn,
  liveTurnToMessage,
  parseCommittedMessage,
  type LiveTurnState,
} from "@/lib/streaming";
import { parseTranscript, type TranscriptMessage } from "@/lib/transcript";
import { useDaemon } from "@/runtime/daemon-provider";

export function useConversation(conversationId: string | undefined) {
  const { client, subscribe } = useDaemon();
  const [conversation, setConversation] = useState<ConversationDto | null>(null);
  const [messages, setMessages] = useState<TranscriptMessage[]>([]);
  const [liveTurn, setLiveTurn] = useState<LiveTurnState | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [pendingPermission, setPendingPermission] = useState<PendingPermission | null>(null);
  const [respondingPermission, setRespondingPermission] = useState(false);
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

      switch (event.kind) {
        case "turn_started":
          setIsRunning(true);
          setLiveTurn(createLiveTurn());
          setPendingPermission(null);
          break;

        case "text_delta":
        case "thought_delta":
        case "tool_call_started":
        case "tool_call_progress":
        case "error":
          setLiveTurn((prev) => {
            const base = prev ?? createLiveTurn();
            return applyHarnessPayload(base, event.payload_json);
          });
          break;

        case "permission_requested": {
          const parsed = parsePermissionRequested(event.payload_json);
          if (parsed) setPendingPermission(parsed);
          break;
        }

        case "permission_resolved":
          setPendingPermission(null);
          setRespondingPermission(false);
          break;

        case "message_committed": {
          const committed = parseCommittedMessage(event.payload_json);
          if (committed) {
            setMessages((prev) => {
              if (prev.some((m) => m.id === committed.id)) return prev;
              return [...prev, committed];
            });
          } else {
            void refresh();
          }
          setLiveTurn(null);
          setIsRunning(false);
          break;
        }

        case "turn_ended":
          setIsRunning(false);
          break;

        default:
          break;
      }
    });
  }, [conversationId, refresh, subscribe]);

  const liveMessage = useMemo(
    () => (liveTurn ? liveTurnToMessage(liveTurn, conversation?.active_harness_id) : null),
    [liveTurn, conversation?.active_harness_id],
  );

  const displayMessages = useMemo(
    () => (liveMessage ? [...messages, liveMessage] : messages),
    [liveMessage, messages],
  );

  const sendMessage = useCallback(
    async (text: string) => {
      if (!conversationId || !text.trim()) return;
      setSending(true);
      setError(null);
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

  const respondPermission = useCallback(
    async (optionId: string) => {
      if (!conversationId || !pendingPermission) return;
      setRespondingPermission(true);
      setError(null);
      try {
        await client.request(method.PERMISSION_RESPOND, {
          conversation_id: conversationId,
          request_id: pendingPermission.requestId,
          option_id: optionId,
        });
      } catch (err) {
        setRespondingPermission(false);
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [client, conversationId, pendingPermission],
  );

  return {
    conversation,
    messages: displayMessages,
    liveMessage,
    loading,
    sending,
    isRunning,
    pendingPermission,
    respondingPermission,
    error,
    refresh,
    sendMessage,
    respondPermission,
  };
}
