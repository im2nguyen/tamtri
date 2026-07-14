import { useCallback, useEffect, useState } from "react";
import { method, type OrchestrationRunDto } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

const ORCHESTRATION_EVENT_KINDS = new Set([
  "orchestration_started",
  "orchestration_step_started",
  "orchestration_forked",
  "orchestration_branch_completed",
  "orchestration_finished",
]);

export function useOrchestrationRun(conversationId: string | undefined, runId: string | null) {
  const { client, subscribe } = useDaemon();
  const [run, setRun] = useState<OrchestrationRunDto | null>(null);
  const [forkedConversationIds, setForkedConversationIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!runId) return;
    setLoading(true);
    try {
      const dto = await client.request<OrchestrationRunDto>(method.ORCHESTRATION_STATUS, {
        run_id: runId,
      });
      setRun(dto);
      setForkedConversationIds(dto.branch_conversation_ids ?? []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client, runId]);

  useEffect(() => {
    if (!runId) {
      setRun(null);
      setForkedConversationIds([]);
      return;
    }
    void refresh();
  }, [refresh, runId]);

  useEffect(() => {
    if (!conversationId || !runId) return;
    return subscribe((event) => {
      if (event.conversation_id !== conversationId) return;
      if (!ORCHESTRATION_EVENT_KINDS.has(event.kind)) return;

      try {
        const payload = JSON.parse(event.payload_json) as {
          run?: OrchestrationRunDto;
          run_id?: string;
          conversation_id?: string;
        };
        if (payload.run_id && payload.run_id !== runId) return;

        if (payload.run) {
          setRun(payload.run);
          setForkedConversationIds(payload.run.branch_conversation_ids ?? []);
          return;
        }

        if (event.kind === "orchestration_forked" && payload.conversation_id) {
          setForkedConversationIds((prev) =>
            prev.includes(payload.conversation_id!) ? prev : [...prev, payload.conversation_id!],
          );
          void refresh();
          return;
        }

        if (event.kind === "orchestration_finished") {
          void refresh();
        }
      } catch {
        void refresh();
      }
    });
  }, [conversationId, refresh, runId, subscribe]);

  const cancel = useCallback(async () => {
    if (!runId) return;
    await client.request(method.ORCHESTRATION_CANCEL, { run_id: runId });
    await refresh();
  }, [client, refresh, runId]);

  const isRunning = run?.status === "running";
  const branchConversationIds =
    forkedConversationIds.length > 0 ? forkedConversationIds : (run?.branch_conversation_ids ?? []);

  return { run, loading, error, refresh, cancel, isRunning, branchConversationIds };
}
