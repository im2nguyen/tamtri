import { FileWarning } from "lucide-react-native";
import { useEffect, useState } from "react";
import { ActivityIndicator, Platform, ScrollView, Text, View } from "react-native";
import { method } from "@tamtri/protocol";

import { SandboxedHtml } from "@/components/transcript/sandboxed-html";
import { useArtifactBytes } from "@/hooks/use-artifact-bytes";
import { useDaemon } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

export interface ArtifactBlock {
  path: string;
  mime_type: string;
  size: number;
  sha256?: string;
  inline?: string;
  integrity_failed?: boolean;
}

interface ArtifactCardProps {
  conversationId: string;
  artifact: ArtifactBlock;
}

function TextPreview({ text, mime }: { text: string; mime: string }) {
  const isMarkdown = mime.includes("markdown") || mime.endsWith("/md");
  return (
    <ScrollView style={{ maxHeight: 320 }}>
      <Text
        style={{
          color: theme.colors.foreground,
          fontSize: theme.fontSize.sm,
          lineHeight: 22,
          fontFamily: isMarkdown ? undefined : "monospace",
        }}
      >
        {text}
      </Text>
    </ScrollView>
  );
}

function CsvPreview({ text }: { text: string }) {
  const rows = text.trim().split(/\r?\n/).slice(0, 12);
  return (
    <ScrollView horizontal style={{ maxHeight: 220 }}>
      <View style={{ gap: 2 }}>
        {rows.map((row, index) => (
          <Text
            key={`${index}-${row.slice(0, 12)}`}
            style={{
              color: theme.colors.foreground,
              fontFamily: "monospace",
              fontSize: theme.fontSize.xs,
            }}
          >
            {row}
          </Text>
        ))}
      </View>
    </ScrollView>
  );
}

function ImagePreview({ bytes, mime, filename }: { bytes: Uint8Array; mime: string; filename: string }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    const blob = new Blob([bytes], { type: mime });
    const url = URL.createObjectURL(blob);
    setSrc(url);
    return () => URL.revokeObjectURL(url);
  }, [bytes, mime]);

  if (Platform.OS !== "web" || !src) {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
        Image preview is available in the web shell.
      </Text>
    );
  }

  return (
    <View style={{ alignItems: "center" }}>
      <img alt={filename} src={src} style={{ maxWidth: "100%", maxHeight: 360, borderRadius: 8 }} />
    </View>
  );
}

export function ArtifactCard({ conversationId, artifact }: ArtifactCardProps) {
  const { client } = useDaemon();
  const filename = artifact.path.split("/").pop() ?? artifact.path;
  const { data, loading, error } = useArtifactBytes(conversationId, artifact);

  if (artifact.integrity_failed) {
    return (
      <View
        style={{
          backgroundColor: theme.colors.surface2,
          borderRadius: theme.radius.xl,
          padding: theme.spacing[4],
          borderWidth: 1,
          borderColor: theme.colors.destructive,
          gap: theme.spacing[2],
        }}
      >
        <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
          <FileWarning color={theme.colors.destructive} size={18} />
          <Text style={{ color: theme.colors.destructive, fontWeight: "700" }}>Integrity check failed</Text>
        </View>
        <Text style={{ color: theme.colors.foreground }}>{filename}</Text>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
          This attachment did not pass hash verification. Its content will not be rendered.
        </Text>
      </View>
    );
  }

  const mime = artifact.mime_type.toLowerCase();

  return (
    <View
      style={{
        backgroundColor: theme.colors.surface2,
        borderRadius: theme.radius.xl,
        padding: theme.spacing[4],
        borderWidth: 1,
        borderColor: theme.colors.border,
        gap: theme.spacing[3],
      }}
    >
      <View>
        <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{filename}</Text>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
          {artifact.mime_type} · {artifact.size.toLocaleString()} bytes
        </Text>
      </View>

      {loading ? <ActivityIndicator color={theme.colors.accentBright} /> : null}
      {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

      {data ? (
        mime.includes("html") ? (
          <SandboxedHtml
            html={data.text}
            title={filename}
            onNavigationBlocked={(url) => {
              void client.request(method.ARTIFACT_LOG_NAVIGATION_BLOCKED, {
                conversation_id: conversationId,
                url,
              });
            }}
          />
        ) : mime.includes("csv") ? (
          <CsvPreview text={data.text} />
        ) : mime.startsWith("text/") || mime.includes("markdown") ? (
          <TextPreview text={data.text} mime={mime} />
        ) : mime.startsWith("image/") && typeof URL !== "undefined" ? (
          <ImagePreview bytes={data.bytes} mime={artifact.mime_type} filename={filename} />
        ) : (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
            Preview is not available for this file type. The artifact is stored in your vault attachments folder.
          </Text>
        )
      ) : null}
    </View>
  );
}
