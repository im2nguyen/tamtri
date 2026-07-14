import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { method } from "@tamtri/protocol";

import type { HarnessProviderEntry } from "@/hooks/use-harness-providers";
import { useDaemon } from "@/runtime/daemon-provider";

export interface ReadinessRecommend {
  agent_id?: string | null;
  display_name: string;
  readiness_state: string;
  recovery_action: string;
  message?: string | null;
  install_doc_url: string;
}

function isReady(entry: HarnessProviderEntry): boolean {
  return entry.enabled && (entry.readiness_state === "ready" || entry.status === "ready");
}

interface ReadinessContextValue {
  entries: HarnessProviderEntry[];
  readyCount: number;
  readyEntries: HarnessProviderEntry[];
  recommendation: ReadinessRecommend | null;
  recommendedEntry: HarnessProviderEntry | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

const ReadinessContext = createContext<ReadinessContextValue | null>(null);

/** Single shared harness readiness probe for the whole app (avoids duplicate RPC + redirect races). */
export function ReadinessProvider({ children }: { children: ReactNode }) {
  const { client } = useDaemon();
  const [entries, setEntries] = useState<HarnessProviderEntry[]>([]);
  const [recommendation, setRecommendation] = useState<ReadinessRecommend | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await client.request<HarnessProviderEntry[]>(method.HARNESS_PROVIDERS_LIST);
      setEntries(rows);

      const recommend = await client.request<ReadinessRecommend>(method.HARNESS_READINESS_RECOMMEND);
      setRecommendation(recommend);
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

  const readyCount = useMemo(() => entries.filter(isReady).length, [entries]);
  const readyEntries = useMemo(() => entries.filter(isReady), [entries]);
  const recommendedEntry = useMemo(() => {
    const agentId = recommendation?.agent_id;
    if (!agentId) return null;
    return entries.find((entry) => entry.id === agentId) ?? null;
  }, [entries, recommendation]);

  const value = useMemo(
    () => ({
      entries,
      readyCount,
      readyEntries,
      recommendation,
      recommendedEntry,
      loading,
      error,
      refresh,
    }),
    [entries, error, loading, readyCount, readyEntries, recommendation, recommendedEntry, refresh],
  );

  return <ReadinessContext.Provider value={value}>{children}</ReadinessContext.Provider>;
}

export function useReadiness(): ReadinessContextValue {
  const ctx = useContext(ReadinessContext);
  if (!ctx) {
    throw new Error("useReadiness must be used within ReadinessProvider");
  }
  return ctx;
}
