import { FileWarning, FolderOpen } from "lucide-react-native";
import { useEffect, useState } from "react";
import { ActivityIndicator, Platform, Pressable, ScrollView, Text, View } from "react-native";
import { method } from "@tamtri/protocol";

import { SandboxedHtml } from "@/components/transcript/sandboxed-html";
import { useArtifactBytes } from "@/hooks/use-artifact-bytes";
import type { ArtifactRef } from "@/lib/artifacts";
import { artifactFilename } from "@/lib/artifacts";
import { revealArtifactInFinder } from "@/lib/artifact-reveal";
import { isDesktopHost } from "@/constants/layout";
import { useDaemon } from "@/runtime/daemon-provider";
import { useTheme } from "@/styles/use-theme";

interface ArtifactPreviewPanelProps {
  conversationId: string;
  artifact: ArtifactRef;
}

function TextPreview({ text, mime }: { text: string; mime: string }) {
  const theme = useTheme();
  const isMarkdown = mime.includes("markdown") || mime.endsWith("/md");
  return (
    <ScrollView style={{ flex: 1 }} contentContainerStyle={{ paddingBottom: theme.spacing[4] }}>
      <Text
        style={{
          color: theme.colors.foreground,
          fontSize: theme.fontSize.sm,
          lineHeight: 22,
          fontFamily: isMarkdown ? undefined : theme.fontFamily.mono,
        }}
      >
        {text}
      </Text>
    </ScrollView>
  );
}

function CsvPreview({ text }: { text: string }) {
  const theme = useTheme();
  const rows = text.trim().split(/\r?\n/);
  return (
    <ScrollView style={{ flex: 1 }} horizontal contentContainerStyle={{ paddingBottom: theme.spacing[4] }}>
      <View style={{ gap: 2 }}>
        {rows.map((row) => (
          <Text
            key={`${row.length}-${row}`}
            style={{
              color: theme.colors.foreground,
              fontFamily: theme.fontFamily.mono,
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
  const theme = useTheme();
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    const blob = new Blob([new Uint8Array(bytes)], { type: mime });
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
    <ScrollView style={{ flex: 1 }} contentContainerStyle={{ alignItems: "center", paddingBottom: theme.spacing[4] }}>
      <img alt={filename} src={src} style={{ maxWidth: "100%", borderRadius: 8 }} />
    </ScrollView>
  );
}

export function ArtifactPreviewPanel({ conversationId, artifact }: ArtifactPreviewPanelProps) {
  const theme = useTheme();
  const { client } = useDaemon();
  const filename = artifactFilename(artifact);
  const { data, loading, error } = useArtifactBytes(conversationId, artifact);
  const mime = artifact.mime_type.toLowerCase();

  if (artifact.integrity_failed) {
    return (
      <View style={{ flex: 1, padding: theme.spacing[4], gap: theme.spacing[2] }}>
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

  return (
    <View style={{ flex: 1, minHeight: 0, gap: theme.spacing[3] }}>
      <View style={{ paddingHorizontal: theme.spacing[4], paddingTop: theme.spacing[2] }}>
        <Text style={{ color: theme.colors.foreground, fontWeight: "600" }} numberOfLines={2}>
          {filename}
        </Text>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
          {artifact.mime_type} · {artifact.size.toLocaleString()} bytes
        </Text>
        {isDesktopHost() ? (
          <Pressable
            onPress={() => void revealArtifactInFinder(client, conversationId, artifact.path)}
            style={{ flexDirection: "row", alignItems: "center", gap: 6, marginTop: theme.spacing[2] }}
          >
            <FolderOpen color={theme.colors.accentBright} size={14} />
            <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>
              Show in Finder
            </Text>
          </Pressable>
        ) : null}
      </View>

      <View style={{ flex: 1, minHeight: 0, paddingHorizontal: theme.spacing[4], paddingBottom: theme.spacing[4] }}>
        {loading ? <ActivityIndicator color={theme.colors.accentBright} /> : null}
        {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

        {data ? (
          mime.includes("html") ? (
            <SandboxedHtml
              html={data.text}
              title={filename}
              fill
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
    </View>
  );
}
