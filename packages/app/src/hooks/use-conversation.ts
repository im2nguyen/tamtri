import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { method, type ConversationDto, type TamtriUIMessage } from "@tamtri/protocol";

import { projectTranscriptToUIMessages, transcriptMessageToUIMessage } from "@/lib/ai-sdk-bridge";
import {
  getCachedConversation,
  patchCachedConversation,
  storeCachedConversation,
} from "@/lib/conversation-cache";
import { parseElicitationRequested, type PendingElicitation } from "@/lib/elicitation";
import { parsePermissionRequested, type PendingPermission } from "@/lib/permissions";
import {
  applyHarnessPayload,
  createLiveTurn,
  liveTurnToMessage,
  parseCommittedMessage,
  type LiveTurnState,
} from "@/lib/streaming";
import { parseTranscript, type TranscriptMessage } from "@/lib/transcript";
import {
  applyUiMessageChunks,
  createLiveUiProjection,
  finalizeLiveUiProjection,
  uiMessageChunksFromUiEvent,
  type LiveUiProjectionState,
} from "@/lib/ui-message-stream";
import { useDaemon } from "@/runtime/daemon-provider";

const STREAMING_EVENT_KINDS = new Set([
  "text_delta",
  "thought_delta",
  "tool_call_started",
  "tool_call_progress",
  "error",
  "permission_requested",
  "permission_resolved",
  "elicitation_requested",
  "elicitation_resolved",
  "orchestration_started",
  "orchestration_step_started",
  "orchestration_forked",
  "orchestration_branch_completed",
  "orchestration_finished",
]);

function resetLiveState(setters: {
  setLiveTurn: (value: LiveTurnState | null) => void;
  setLiveUiProjection: (value: LiveUiProjectionState | null) => void;
  setIsRunning: (value: boolean) => void;
  setPendingPermission: (value: PendingPermission | null) => void;
  setPendingElicitation: (value: PendingElicitation | null) => void;
  setActiveMode: (value: string | null) => void;
}) {
  setters.setLiveTurn(null);
  setters.setLiveUiProjection(null);
  setters.setIsRunning(false);
  setters.setPendingPermission(null);
  setters.setPendingElicitation(null);
  setters.setActiveMode(null);
}

export function useConversation(conversationId: string | undefined) {
  const { client, subscribe } = useDaemon();
  const [conversation, setConversation] = useState<ConversationDto | null>(null);
  const [messages, setMessages] = useState<TranscriptMessage[]>([]);
  const [uiMessages, setUiMessages] = useState<TamtriUIMessage[]>([]);
  const [liveTurn, setLiveTurn] = useState<LiveTurnState | null>(null);
  const [liveUiProjection, setLiveUiProjection] = useState<LiveUiProjectionState | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [pendingPermission, setPendingPermission] = useState<PendingPermission | null>(null);
  const [respondingPermission, setRespondingPermission] = useState(false);
  const [pendingElicitation, setPendingElicitation] = useState<PendingElicitation | null>(null);
  const [respondingElicitation, setRespondingElicitation] = useState(false);
  const [activeMode, setActiveMode] = useState<string | null>(null);
  const [loading, setLoading] = useState(Boolean(conversationId));
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeConversationId = useRef<string | undefined>(conversationId);

  const applyLoadedConversation = useCallback(
    (id: string, dto: ConversationDto, parsed: TranscriptMessage[], ui: TamtriUIMessage[]) => {
      if (activeConversationId.current !== id) return;
      setConversation(dto);
      setMessages(parsed);
      setUiMessages(ui);
      storeCachedConversation(id, {
        conversation: dto,
        messages: parsed,
        uiMessages: ui,
      });
    },
    [],
  );

  const refresh = useCallback(async () => {
    if (!conversationId) return;
    const requestId = conversationId;
    try {
      const dto = await client.request<ConversationDto>(method.CONVERSATION_LOAD, {
        id: requestId,
      });
      const parsed = parseTranscript(dto.transcript_json);
      const ui = projectTranscriptToUIMessages(parsed);
      applyLoadedConversation(requestId, dto, parsed, ui);
      setError(null);
    } catch (err) {
      if (activeConversationId.current === requestId) {
        setError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      if (activeConversationId.current === requestId) {
        setLoading(false);
      }
    }
  }, [applyLoadedConversation, client, conversationId]);

  useEffect(() => {
    activeConversationId.current = conversationId;

    if (!conversationId) {
      setConversation(null);
      setMessages([]);
      setUiMessages([]);
      setLoading(false);
      resetLiveState({
        setLiveTurn,
        setLiveUiProjection,
        setIsRunning,
        setPendingPermission,
        setPendingElicitation,
        setActiveMode,
      });
      return;
    }

    resetLiveState({
      setLiveTurn,
      setLiveUiProjection,
      setIsRunning,
      setPendingPermission,
      setPendingElicitation,
      setActiveMode,
    });

    const cached = getCachedConversation(conversationId);
    if (cached) {
      setConversation(cached.conversation);
      setMessages(cached.messages);
      setUiMessages(cached.uiMessages);
      setLoading(false);
    } else {
      setConversation(null);
      setMessages([]);
      setUiMessages([]);
      setLoading(true);
    }

    void refresh();
  }, [conversationId, refresh]);

  useEffect(() => {
    if (!conversationId) return;
    return subscribe((event) => {
      if (event.conversation_id !== conversationId) return;

      switch (event.kind) {
        case "turn_started":
          setIsRunning(true);
          setLiveTurn(createLiveTurn());
          setLiveUiProjection(createLiveUiProjection(conversation?.active_harness_id));
          setPendingPermission(null);
          setPendingElicitation(null);
          break;

        case "mode_changed": {
          try {
            const payload = JSON.parse(event.payload_json) as Record<string, unknown>;
            const mode = typeof payload.mode === "string" ? payload.mode : null;
            setActiveMode(mode);
          } catch {
            /* ignore malformed mode payload */
          }
          break;
        }

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

        case "elicitation_requested": {
          const parsed = parseElicitationRequested(event.payload_json);
          if (parsed) setPendingElicitation(parsed);
          break;
        }

        case "elicitation_resolved":
          setPendingElicitation(null);
          setRespondingElicitation(false);
          break;

        case "message_committed": {
          const committed = parseCommittedMessage(event.payload_json);
          if (committed) {
            setMessages((prev) => {
              if (prev.some((m) => m.id === committed.id)) return prev;
              const next = [...prev, committed];
              patchCachedConversation(conversationId, { messages: next });
              return next;
            });
            setUiMessages((prev) => {
              if (prev.some((m) => m.id === committed.id)) return prev;
              const next = [...prev, transcriptMessageToUIMessage(committed)];
              patchCachedConversation(conversationId, { uiMessages: next });
              return next;
            });
          } else {
            void refresh();
          }
          setLiveTurn(null);
          setLiveUiProjection(null);
          setIsRunning(false);
          break;
        }

        case "turn_ended":
          setLiveUiProjection((prev) => (prev ? finalizeLiveUiProjection(prev) : prev));
          setIsRunning(false);
          break;

        default:
          break;
      }

      if (STREAMING_EVENT_KINDS.has(event.kind)) {
        const chunks = uiMessageChunksFromUiEvent(event);
        if (chunks?.length) {
          setLiveUiProjection((prev) => {
            const base =
              prev ?? createLiveUiProjection(conversation?.active_harness_id);
            return applyUiMessageChunks(base, chunks);
          });
        }
      }
    });
  }, [conversation?.active_harness_id, conversationId, refresh, subscribe]);

  const liveMessage = useMemo(
    () => (liveTurn ? liveTurnToMessage(liveTurn, conversation?.active_harness_id) : null),
    [liveTurn, conversation?.active_harness_id],
  );

  const liveUiMessage = useMemo(
    () => liveUiProjection?.message ?? null,
    [liveUiProjection],
  );

  const displayMessages = useMemo(
    () => (liveMessage ? [...messages, liveMessage] : messages),
    [liveMessage, messages],
  );

  const displayUiMessages = useMemo(
    () => (liveUiMessage ? [...uiMessages, liveUiMessage] : uiMessages),
    [liveUiMessage, uiMessages],
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

  const cancelRun = useCallback(async () => {
    if (!conversationId) return;
    setError(null);
    try {
      await client.request(method.RUN_CANCEL, { conversation_id: conversationId });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [client, conversationId]);

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

  const respondElicitation = useCallback(
    async (action: "accept" | "decline" | "cancel", dataJson?: string) => {
      if (!conversationId || !pendingElicitation) return;
      setRespondingElicitation(true);
      setError(null);
      try {
        await client.request(method.ELICITATION_RESPOND, {
          conversation_id: conversationId,
          request_id: pendingElicitation.requestId,
          action,
          data_json: dataJson,
        });
      } catch (err) {
        setRespondingElicitation(false);
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [client, conversationId, pendingElicitation],
  );

  const setModel = useCallback(
    async (modelId: string) => {
      if (!conversationId || !modelId.trim()) return;
      setError(null);
      try {
        const dto = await client.request<ConversationDto>(method.CONVERSATION_SET_MODEL, {
          id: conversationId,
          model_id: modelId.trim(),
        });
        if (activeConversationId.current !== conversationId) return;
        setConversation(dto);
        patchCachedConversation(conversationId, { conversation: dto });
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      }
    },
    [client, conversationId],
  );

  return {
    conversation,
    messages: displayMessages,
    uiMessages: displayUiMessages,
    liveMessage,
    liveUiMessage,
    loading,
    sending,
    isRunning,
    pendingPermission,
    respondingPermission,
    pendingElicitation,
    respondingElicitation,
    activeMode,
    error,
    refresh,
    sendMessage,
    cancelRun,
    respondPermission,
    respondElicitation,
    setModel,
  };
}
