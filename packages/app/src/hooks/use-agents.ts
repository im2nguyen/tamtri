import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export interface AgentRosterEntry {
  id: string;
  display_name: string;
  runtime_model_switch?: boolean;
}

export interface ModelEntry {
  id: string;
  display_name: string;
}

export function useAgents() {
  const { client } = useDaemon();
  const [agents, setAgents] = useState<AgentRosterEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const rows = await client.request<AgentRosterEntry[]>(method.AGENTS_LIST);
        setAgents(rows);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    })();
  }, [client]);

  const loadModels = useCallback(
    async (agentId: string): Promise<ModelEntry[]> => {
      const rows = await client.request<ModelEntry[]>(method.AGENTS_MODELS, { agent_id: agentId });
      return rows;
    },
    [client],
  );

  return { agents, loading, error, loadModels };
}
