import { ExternalLink, FileText, FileWarning } from "lucide-react-native";
import { Pressable, Text, View } from "react-native";

import type { ArtifactRef } from "@/lib/artifacts";
import { artifactFilename, artifactKey } from "@/lib/artifacts";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

export type ArtifactBlock = ArtifactRef;

interface ArtifactCardProps {
  conversationId: string;
  artifact: ArtifactBlock;
}

export function ArtifactCard({ conversationId, artifact }: ArtifactCardProps) {
  const theme = useTheme();
  const filename = artifactFilename(artifact);
  const openArtifactPreview = useUiStore((s) => s.openArtifactPreview);
  const selectedArtifact = useUiStore((s) => s.selectedArtifact);
  const artifactSidebarOpen = useUiStore((s) => s.artifactSidebarOpen);
  const selected =
    artifactSidebarOpen &&
    selectedArtifact !== null &&
    artifactKey(selectedArtifact) === artifactKey(artifact);

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

  return (
    <Pressable
      onPress={() => openArtifactPreview(conversationId, artifact)}
      style={({ pressed }) => ({
        backgroundColor: pressed ? theme.colors.surface3 : theme.colors.surface2,
        borderRadius: theme.radius.xl,
        padding: theme.spacing[4],
        borderWidth: 1,
        borderColor: selected ? theme.colors.accent : theme.colors.border,
        gap: theme.spacing[2],
      })}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[3] }}>
        <View
          style={{
            width: 36,
            height: 36,
            borderRadius: theme.radius.lg,
            backgroundColor: theme.colors.surface1,
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <FileText color={theme.colors.accentBright} size={18} />
        </View>
        <View style={{ flex: 1, minWidth: 0 }}>
          <Text style={{ color: theme.colors.foreground, fontWeight: "600" }} numberOfLines={1}>
            {filename}
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
            {artifact.mime_type} · {artifact.size.toLocaleString()} bytes
          </Text>
        </View>
        <ExternalLink color={theme.colors.foregroundMuted} size={16} />
      </View>
      <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
        {selected ? "Previewing in sidebar" : "Click to preview in sidebar"}
      </Text>
    </Pressable>
  );
}
