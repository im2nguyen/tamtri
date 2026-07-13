import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { encodeBase64 } from "@/lib/base64";
import type { WorkdirFile } from "@/lib/daemon-types";
import { electronFilePath, shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";

export function useWorkdir(conversationId: string | undefined) {
  const { client } = useDaemon();
  const [files, setFiles] = useState<WorkdirFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!conversationId) {
      setFiles([]);
      return;
    }
    setLoading(true);
    try {
      const rows = await client.request<WorkdirFile[]>(method.WORKDIR_LIST_FILES, {
        conversation_id: conversationId,
      });
      setFiles(rows);
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

  const attachPaths = useCallback(
    async (paths: string[]) => {
      if (!conversationId || paths.length === 0) return [];
      const attached = await Promise.all(
        paths.map((sourcePath) =>
          client.request<string>(method.WORKDIR_COPY_FILE, {
            conversation_id: conversationId,
            source_path: sourcePath,
          }),
        ),
      );
      await refresh();
      return attached;
    },
    [client, conversationId, refresh],
  );

  const attachBrowserFiles = useCallback(
    async (browserFiles: File[]) => {
      if (!conversationId || browserFiles.length === 0) return [];
      const attached = await Promise.all(
        browserFiles.map(async (file) => {
          const electronPath = electronFilePath(file);
          if (electronPath) {
            return client.request<string>(method.WORKDIR_COPY_FILE, {
              conversation_id: conversationId,
              source_path: electronPath,
            });
          }
          const buffer = new Uint8Array(await file.arrayBuffer());
          return client.request<string>(method.WORKDIR_WRITE_FILE, {
            conversation_id: conversationId,
            filename: file.name,
            data_base64: encodeBase64(buffer),
          });
        }),
      );
      await refresh();
      return attached;
    },
    [client, conversationId, refresh],
  );

  const pickAndAttach = useCallback(async () => {
    const shell = shellBridge();
    if (!shell?.pickOpenFile) return [];
    const path = await shell.pickOpenFile({ title: "Attach to conversation" });
    if (!path) return [];
    return attachPaths([path]);
  }, [attachPaths]);

  return {
    files,
    loading,
    error,
    refresh,
    attachPaths,
    attachBrowserFiles,
    pickAndAttach,
    canPickFile: Boolean(shellBridge()?.pickOpenFile),
  };
}
