import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import type { SearchHit } from "@/lib/daemon-types";
import { useDaemon } from "@/runtime/daemon-provider";

export function useSearch() {
  const { client } = useDaemon();
  const [scopeMessage, setScopeMessage] = useState<string>(
    "Search covers conversation titles and transcript text.",
  );
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const message = await client.request<string>(method.SEARCH_SCOPE_MESSAGE);
        if (message) setScopeMessage(message);
      } catch {
        // keep default copy
      }
    })();
  }, [client]);

  const search = useCallback(
    async (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) {
        setHits([]);
        return [];
      }
      setLoading(true);
      try {
        const rows = await client.request<SearchHit[]>(method.SEARCH_CONVERSATIONS, { query: trimmed });
        setHits(rows);
        setError(null);
        return rows;
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setHits([]);
        return [];
      } finally {
        setLoading(false);
      }
    },
    [client],
  );

  return { hits, loading, error, scopeMessage, search };
}
