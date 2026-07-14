import type { TamtriUIMessage } from "@tamtri/protocol";

export interface ArtifactRef {
  path: string;
  mime_type: string;
  size: number;
  sha256?: string;
  inline?: string;
  integrity_failed?: boolean;
}

export function artifactKey(artifact: ArtifactRef): string {
  return artifact.sha256 ? `${artifact.path}:${artifact.sha256}` : artifact.path;
}

export function collectArtifactsFromUiMessages(messages: TamtriUIMessage[]): ArtifactRef[] {
  const seen = new Set<string>();
  const artifacts: ArtifactRef[] = [];

  for (const message of messages) {
    for (const part of message.parts) {
      if (part.type !== "data-tamtri-artifact") continue;
      const artifact: ArtifactRef = {
        path: part.data.path,
        mime_type: part.data.mime_type,
        size: part.data.size,
        sha256: part.data.sha256,
        inline: part.data.inline,
        integrity_failed: part.data.integrity_failed,
      };
      const key = artifactKey(artifact);
      if (seen.has(key)) continue;
      seen.add(key);
      artifacts.push(artifact);
    }
  }

  return artifacts;
}

export function artifactFilename(artifact: ArtifactRef): string {
  return artifact.path.split("/").pop() ?? artifact.path;
}
