import { method } from "@tamtri/protocol";

import { shellBridge } from "@/lib/shell";
import type { DaemonClient } from "@tamtri/client";

export async function revealArtifactInFinder(
  client: DaemonClient,
  conversationId: string,
  attachmentPath: string,
): Promise<void> {
  try {
    const resolved = await client.request<string>(method.ARTIFACT_RESOLVE_PATH, {
      conversation_id: conversationId,
      path: attachmentPath,
    });
    await shellBridge()?.showItemInFolder?.(resolved);
    return;
  } catch {
    /* fall through */
  }

  try {
    const folder = await client.request<string>(method.CONVERSATION_FOLDER_PATH, {
      conversation_id: conversationId,
    });
    await shellBridge()?.showItemInFolder?.(folder);
  } catch {
    /* ignore */
  }
}
