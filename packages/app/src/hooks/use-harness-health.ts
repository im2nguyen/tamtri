import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export interface HarnessHealthEntry {
  id: string;
  display_name: string;
  command: string;
  status: "ready" | "missing" | "unknown" | string;
  install_doc_url: string;
}

export function useHarnessHealth() {
  const { client } = useDaemon();
  const [entries, setEntries] = useState<HarnessHealthEntry[]>([]);
  const [checklist, setChecklist] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [rows, text] = await Promise.all([
        client.request<HarnessHealthEntry[]>(method.HARNESS_HEALTH_LIST),
        client.request<string>(method.HARNESS_HEALTH_CHECKLIST),
      ]);
      setEntries(rows);
      setChecklist(text);
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

  return { entries, checklist, loading, error, refresh };
}
