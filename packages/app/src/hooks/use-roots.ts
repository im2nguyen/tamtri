import { useCallback, useEffect, useState } from "react";
import { method, type RootDto } from "@tamtri/protocol";

import { shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";

export function useRoots(conversationId: string | undefined) {
  const { client } = useDaemon();
  const [roots, setRoots] = useState<RootDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!conversationId) {
      setRoots([]);
      return;
    }
    setLoading(true);
    try {
      const rows = await client.request<RootDto[]>(method.ROOTS_LIST, {
        conversation_id: conversationId,
      });
      setRoots(rows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client, conversationId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const attachFilesystemRoot = useCallback(async () => {
    if (!conversationId) return null;
    const shell = shellBridge();
    if (!shell?.pickOpenFile) return null;
    const path = await shell.pickOpenFile({ title: "Attach folder as root" });
    if (!path) return null;
    const name = path.split(/[/\\]/).pop() ?? "Root";
    const root = await client.request<RootDto>(method.ROOTS_ATTACH, {
      conversation_id: conversationId,
      name,
      uri: path,
      kind: "filesystem",
      scope: "conversation",
    });
    await refresh();
    return root;
  }, [client, conversationId, refresh]);

  return {
    roots,
    loading,
    error,
    refresh,
    attachFilesystemRoot,
    canAttachRoot: Boolean(conversationId && shellBridge()?.pickOpenFile),
  };
}
