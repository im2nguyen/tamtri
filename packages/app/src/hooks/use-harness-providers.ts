import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export interface HarnessProviderEntry {
  id: string;
  display_name: string;
  command: string;
  status: "ready" | "missing" | "unknown" | "disabled" | string;
  readiness_state?: string;
  recovery_action?: string;
  readiness_message?: string | null;
  install_doc_url: string;
  adapter_type: "acp" | "native" | string;
  adapter_kind: string;
  enabled: boolean;
  model_count?: number | null;
}

export function useHarnessProviders() {
  const { client } = useDaemon();
  const [entries, setEntries] = useState<HarnessProviderEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await client.request<HarnessProviderEntry[]>(method.HARNESS_PROVIDERS_LIST);
      setEntries(rows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client]);

  const setEnabled = useCallback(
    async (agentId: string, enabled: boolean) => {
      setPendingId(agentId);
      try {
        await client.request(method.HARNESS_ROSTER_SET_ENABLED, { agent_id: agentId, enabled });
        setEntries((current) =>
          current.map((entry) => (entry.id === agentId ? { ...entry, enabled } : entry)),
        );
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      } finally {
        setPendingId((current) => (current === agentId ? null : current));
      }
    },
    [client],
  );

  const addProvider = useCallback(
    async (payload: {
      id: string;
      display_name: string;
      command: string;
      args: string[];
      env?: { name: string; value: string }[];
      adapter?: string;
    }) => {
      setPendingId(payload.id);
      try {
        await client.request(method.HARNESS_ROSTER_ADD, {
          ...payload,
          adapter: payload.adapter ?? "acp",
        });
        await refresh();
      } finally {
        setPendingId((current) => (current === payload.id ? null : current));
      }
    },
    [client, refresh],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { entries, loading, error, pendingId, refresh, setEnabled, addProvider };
}
