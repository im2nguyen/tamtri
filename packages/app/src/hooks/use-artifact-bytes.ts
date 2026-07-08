import { useCallback, useEffect, useState } from "react";
import { method } from "@tamtri/protocol";

import { bytesToText, decodeBase64 } from "@/lib/base64";
import { useDaemon } from "@/runtime/daemon-provider";

export interface ArtifactBytes {
  bytes: Uint8Array;
  text: string;
}

export function useArtifactBytes(
  conversationId: string | undefined,
  artifact: {
    path: string;
    size: number;
    sha256?: string;
    inline?: string;
    integrity_failed?: boolean;
  } | null,
) {
  const { client } = useDaemon();
  const [data, setData] = useState<ArtifactBytes | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!conversationId || !artifact || artifact.integrity_failed) {
      setData(null);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      if (artifact.inline && artifact.sha256) {
        await client.request<null>(method.ARTIFACT_VERIFY_INLINE, {
          size: artifact.size,
          sha256: artifact.sha256,
          inline_content: artifact.inline,
        });
        const bytes = new TextEncoder().encode(artifact.inline);
        setData({ bytes, text: artifact.inline });
        return;
      }

      if (!artifact.sha256) {
        throw new Error("Artifact is missing integrity metadata.");
      }

      const result = await client.request<{ data_base64: string }>(method.ATTACHMENT_READ_VERIFIED, {
        conversation_id: conversationId,
        path: artifact.path,
        size: artifact.size,
        sha256: artifact.sha256,
      });
      const bytes = decodeBase64(result.data_base64);
      setData({ bytes, text: bytesToText(bytes) });
    } catch (err) {
      setData(null);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [artifact, client, conversationId]);

  useEffect(() => {
    void load();
  }, [load]);

  return { data, loading, error, reload: load };
}
