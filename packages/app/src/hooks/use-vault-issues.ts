import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import type { VaultIssue } from "@/lib/daemon-types";
import { useDaemon } from "@/runtime/daemon-provider";

export function useVaultIssues() {
  const { client } = useDaemon();
  const [issues, setIssues] = useState<VaultIssue[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await client.request<VaultIssue[]>(method.VAULT_ISSUES);
      setIssues(rows);
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

  return { issues, loading, error, refresh };
}
