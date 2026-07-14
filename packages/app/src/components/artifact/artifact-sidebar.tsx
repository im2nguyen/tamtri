import { FileText, X } from "lucide-react-native";
import { Modal, Pressable, ScrollView, Text, useWindowDimensions, View } from "react-native";
import Animated from "react-native-reanimated";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { ArtifactPreviewPanel } from "@/components/artifact/artifact-preview-panel";
import { SidebarResizeHandle } from "@/components/layout/sidebar-resize-handle";
import {
  ARTIFACT_SIDEBAR_WIDTH,
  clampArtifactSidebarWidth,
  isArtifactSidebarInline,
} from "@/constants/layout";
import { useResizableSidebarWidth } from "@/hooks/use-resizable-sidebar-width";
import { artifactFilename, artifactKey, type ArtifactRef } from "@/lib/artifacts";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface ArtifactSidebarProps {
  conversationId: string;
  artifacts: ArtifactRef[];
}

function ArtifactListRow({
  artifact,
  selected,
  onPress,
}: {
  artifact: ArtifactRef;
  selected: boolean;
  onPress: () => void;
}) {
  const theme = useTheme();
  const filename = artifactFilename(artifact);
  return (
    <Pressable
      onPress={onPress}
      style={({ pressed }) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[2],
        paddingHorizontal: theme.spacing[3],
        paddingVertical: theme.spacing[2],
        borderRadius: theme.radius.md,
        backgroundColor: selected
          ? theme.colors.surface3
          : pressed
            ? theme.colors.surfaceSidebarHover
            : "transparent",
        borderWidth: 1,
        borderColor: selected ? theme.colors.accent : "transparent",
      })}
    >
      <FileText color={selected ? theme.colors.accentBright : theme.colors.foregroundMuted} size={14} />
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text numberOfLines={1} style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>
          {filename}
        </Text>
        <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }}>
          {artifact.mime_type}
        </Text>
      </View>
    </Pressable>
  );
}

function ArtifactSidebarContent({ conversationId, artifacts }: ArtifactSidebarProps) {
  const theme = useTheme();
  const insets = useSafeAreaInsets();
  const selectedArtifact = useUiStore((s) => s.selectedArtifact);
  const openArtifactPreview = useUiStore((s) => s.openArtifactPreview);
  const closeArtifactPreview = useUiStore((s) => s.closeArtifactPreview);
  const showList = artifacts.length > 1;

  return (
    <View
      style={{
        flex: 1,
        backgroundColor: theme.colors.surfaceSidebar,
        borderLeftWidth: 1,
        borderLeftColor: theme.colors.border,
      }}
    >
      <View
        style={{
          height: theme.layout.headerHeight + insets.top,
          paddingTop: insets.top,
          paddingHorizontal: theme.spacing[3],
          borderBottomWidth: 1,
          borderBottomColor: theme.colors.border,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[2],
        }}
      >
        <Text style={{ flex: 1, color: theme.colors.foreground, fontWeight: "600", fontSize: theme.fontSize.sm }}>
          Artifacts
        </Text>
        <Pressable onPress={closeArtifactPreview} hitSlop={8}>
          <X color={theme.colors.foregroundMuted} size={18} />
        </Pressable>
      </View>

      {showList ? (
        <View
          style={{
            borderBottomWidth: 1,
            borderBottomColor: theme.colors.border,
            paddingVertical: theme.spacing[2],
            paddingHorizontal: theme.spacing[2],
            maxHeight: 160,
          }}
        >
          <ScrollView>
            <View style={{ gap: theme.spacing[1] }}>
              {artifacts.map((artifact) => (
                <ArtifactListRow
                  key={artifactKey(artifact)}
                  artifact={artifact}
                  selected={selectedArtifact ? artifactKey(selectedArtifact) === artifactKey(artifact) : false}
                  onPress={() => openArtifactPreview(conversationId, artifact)}
                />
              ))}
            </View>
          </ScrollView>
        </View>
      ) : null}

      {selectedArtifact ? (
        <ArtifactPreviewPanel conversationId={conversationId} artifact={selectedArtifact} />
      ) : (
        <View style={{ flex: 1, alignItems: "center", justifyContent: "center", padding: theme.spacing[6] }}>
          <FileText color={theme.colors.foregroundMuted} size={28} />
          <Text
            style={{
              color: theme.colors.foregroundMuted,
              fontSize: theme.fontSize.sm,
              textAlign: "center",
              marginTop: theme.spacing[3],
              lineHeight: 20,
            }}
          >
            Select an artifact from the transcript or list to preview it here.
          </Text>
        </View>
      )}
    </View>
  );
}

export function ArtifactSidebar({ conversationId, artifacts }: ArtifactSidebarProps) {
  const { width } = useWindowDimensions();
  const inline = isArtifactSidebarInline(width);
  const open = useUiStore((s) => s.artifactSidebarOpen);
  const artifactSidebarWidth = useUiStore((s) => s.artifactSidebarWidth);
  const setArtifactSidebarWidth = useUiStore((s) => s.setArtifactSidebarWidth);
  const closeArtifactPreview = useUiStore((s) => s.closeArtifactPreview);
  const { animatedStyle, resizeGesture, handleCursorStyle } = useResizableSidebarWidth({
    width: artifactSidebarWidth,
    setWidth: setArtifactSidebarWidth,
    clampWidth: clampArtifactSidebarWidth,
    edge: "left",
    enabled: inline,
  });

  if (!open) return null;

  if (inline) {
    return (
      <Animated.View style={[{ flexShrink: 0, alignSelf: "stretch" }, animatedStyle]}>
        <View style={{ flex: 1, position: "relative" }}>
          <SidebarResizeHandle edge="left" gesture={resizeGesture} cursorStyle={handleCursorStyle} />
          <ArtifactSidebarContent conversationId={conversationId} artifacts={artifacts} />
        </View>
      </Animated.View>
    );
  }

  return (
    <Modal visible transparent animationType="fade" onRequestClose={closeArtifactPreview}>
      <View style={{ flex: 1, flexDirection: "row", backgroundColor: "rgba(0,0,0,0.55)" }}>
        <Pressable style={{ flex: 1 }} onPress={closeArtifactPreview} />
        <View style={{ width: Math.min(width * 0.92, ARTIFACT_SIDEBAR_WIDTH + 40), maxWidth: width, flex: 1 }}>
          <ArtifactSidebarContent conversationId={conversationId} artifacts={artifacts} />
        </View>
      </View>
    </Modal>
  );
}
