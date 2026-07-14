import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export interface HarnessPickerSettings {
  harness_order: string[];
  hidden_harness_ids: string[];
  enable_cli_update_checks: boolean;
}

export function useHarnessPickerSettings() {
  const { client } = useDaemon();
  const [settings, setSettings] = useState<HarnessPickerSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const payload = await client.request<HarnessPickerSettings>(method.HARNESS_PICKER_SETTINGS_GET);
      setSettings(payload);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client]);

  const save = useCallback(
    async (next: HarnessPickerSettings) => {
      setPending(true);
      try {
        await client.request(method.HARNESS_PICKER_SETTINGS_SET, next);
        setSettings(next);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      } finally {
        setPending(false);
      }
    },
    [client],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { settings, loading, error, pending, refresh, save };
}
