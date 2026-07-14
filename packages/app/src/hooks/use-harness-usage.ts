import { useCallback, useEffect, useState } from "react";
import { method, type HarnessUsageListDto } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export type HarnessUsageView =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; payload: HarnessUsageListDto; isRefreshing: boolean }
  | { kind: "unavailable"; message: string };

export function useHarnessUsage() {
  const { serverInfo, client } = useDaemon();
  const supportsUsage = serverInfo?.features?.provider_usage === true;
  const [view, setView] = useState<HarnessUsageView>({ kind: "loading" });

  const refresh = useCallback(async () => {
    if (!supportsUsage) {
      setView({
        kind: "unavailable",
        message: "Usage quotas require a newer tamtri daemon.",
      });
      return;
    }
    setView((current) =>
      current.kind === "ready"
        ? { ...current, isRefreshing: true }
        : { kind: "loading" },
    );
    try {
      const payload = await client.request<HarnessUsageListDto>(method.HARNESS_USAGE_LIST);
      setView({ kind: "ready", payload, isRefreshing: false });
    } catch (err) {
      setView({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [client, supportsUsage]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { view, refresh, supportsUsage };
}
