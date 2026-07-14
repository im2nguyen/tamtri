import { AppWindow, CheckCircle2, FileText, ListChecks, X } from "lucide-react-native";
import { useEffect, useState } from "react";
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
import type { RightDockState, RightDockTabId } from "@/lib/conversation-surface";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface RightDockProps {
  conversationId: string;
  artifacts: ArtifactRef[];
  state: RightDockState;
}

function TabButton({
  label,
  count,
  selected,
  onPress,
}: {
  label: string;
  count: number;
  selected: boolean;
  onPress: () => void;
}) {
  const theme = useTheme();
  return (
    <Pressable
      onPress={onPress}
      accessibilityRole="tab"
      accessibilityState={{ selected }}
      style={({ pressed }) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: 5,
        height: 26,
        paddingHorizontal: 9,
        borderRadius: theme.radius.md,
        backgroundColor: selected ? theme.colors.surface3 : pressed ? theme.colors.surface2 : "transparent",
      })}
    >
      <Text style={{ color: selected ? theme.colors.foreground : theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
        {label}
      </Text>
      <Text style={{ color: theme.colors.foregroundMuted, opacity: 0.7, fontSize: theme.fontSize.xs }}>{count}</Text>
    </Pressable>
  );
}

function ArtifactList({
  conversationId,
  artifacts,
}: {
  conversationId: string;
  artifacts: ArtifactRef[];
}) {
  const theme = useTheme();
  const selectedArtifact = useUiStore((s) => s.selectedArtifact);
  const openArtifactPreview = useUiStore((s) => s.openArtifactPreview);
  if (artifacts.length <= 1) return null;
  return (
    <ScrollView style={{ maxHeight: 132 }} contentContainerStyle={{ padding: theme.spacing[2], gap: 3 }}>
      {artifacts.map((artifact) => {
        const selected = selectedArtifact
          ? artifactKey(selectedArtifact) === artifactKey(artifact)
          : false;
        return (
          <Pressable
            key={artifactKey(artifact)}
            onPress={() => openArtifactPreview(conversationId, artifact)}
            style={({ pressed }) => ({
              minHeight: 30,
              paddingHorizontal: 8,
              borderRadius: theme.radius.md,
              flexDirection: "row",
              alignItems: "center",
              gap: 7,
              backgroundColor: selected
                ? theme.colors.surface3
                : pressed
                  ? theme.colors.surfaceSidebarHover
                  : "transparent",
            })}
          >
            <FileText color={theme.colors.foregroundMuted} size={13} />
            <Text numberOfLines={1} style={{ flex: 1, color: theme.colors.foreground, fontSize: theme.fontSize.xs }}>
              {artifactFilename(artifact)}
            </Text>
          </Pressable>
        );
      })}
    </ScrollView>
  );
}

function SummaryTab({
  state,
  activeTab,
}: {
  state: RightDockState;
  activeTab: Exclude<RightDockTabId, "artifacts">;
}) {
  const theme = useTheme();
  const entries =
    activeTab === "apps"
      ? state.apps.map((app) => ({
          id: app.uri,
          title: app.uri,
          meta: app.serverId ? `From ${app.serverId}` : "MCP App",
          icon: <AppWindow color={theme.colors.accentBright} size={15} />,
        }))
      : state.tasks.map((task) => ({
          id: task.taskId,
          title: task.title ?? task.taskId,
          meta: task.resultSummary ?? task.status,
          icon: <CheckCircle2 color={theme.colors.accentBright} size={15} />,
        }));
  return (
    <ScrollView contentContainerStyle={{ padding: theme.spacing[3], gap: theme.spacing[2] }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
        {activeTab === "apps"
          ? "App summaries stay here while their sandboxed interactive view remains in the transcript."
          : "Task status is summarized here. Full task controls remain in the transcript."}
      </Text>
      {entries.map((entry) => (
        <View
          key={entry.id}
          accessible
          accessibilityLabel={`${entry.title}, ${entry.meta}`}
          style={{
            borderTopWidth: 1,
            borderColor: theme.colors.border,
            paddingVertical: theme.spacing[2],
            flexDirection: "row",
            alignItems: "flex-start",
            gap: theme.spacing[2],
          }}
        >
          {entry.icon}
          <View style={{ flex: 1, minWidth: 0 }}>
            <Text numberOfLines={2} style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>
              {entry.title}
            </Text>
            <Text numberOfLines={2} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }}>
              {entry.meta}
            </Text>
          </View>
        </View>
      ))}
    </ScrollView>
  );
}

function RightDockContent({ conversationId, artifacts, state }: RightDockProps) {
  const theme = useTheme();
  const insets = useSafeAreaInsets();
  const close = useUiStore((s) => s.closeArtifactPreview);
  const selectedArtifact = useUiStore((s) => s.selectedArtifact);
  const [activeTab, setActiveTab] = useState<RightDockTabId>(state.tabs[0]?.id ?? "artifacts");

  useEffect(() => {
    if (!state.tabs.some((tab) => tab.id === activeTab)) {
      setActiveTab(state.tabs[0]?.id ?? "artifacts");
    }
  }, [activeTab, state.tabs]);

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surfaceSidebar, borderLeftWidth: 1, borderLeftColor: theme.colors.border }}>
      <View
        style={{
          height: 46 + insets.top,
          paddingTop: insets.top,
          paddingHorizontal: theme.spacing[3],
          borderBottomWidth: 1,
          borderBottomColor: theme.colors.border,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[2],
        }}
      >
        <View accessibilityRole="tablist" style={{ flex: 1, flexDirection: "row", alignItems: "center", gap: 3 }}>
          {state.tabs.map((tab) => (
            <TabButton
              key={tab.id}
              label={tab.label}
              count={tab.count}
              selected={tab.id === activeTab}
              onPress={() => setActiveTab(tab.id)}
            />
          ))}
        </View>
        <Pressable accessibilityRole="button" accessibilityLabel="Close right dock" onPress={close} hitSlop={8}>
          <X color={theme.colors.foregroundMuted} size={16} />
        </Pressable>
      </View>

      {activeTab === "artifacts" ? (
        <>
          <ArtifactList conversationId={conversationId} artifacts={artifacts} />
          {selectedArtifact ? (
            <ArtifactPreviewPanel conversationId={conversationId} artifact={selectedArtifact} />
          ) : (
            <View style={{ flex: 1, alignItems: "center", justifyContent: "center", padding: theme.spacing[6] }}>
              <ListChecks color={theme.colors.foregroundMuted} size={24} />
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, textAlign: "center", marginTop: 10 }}>
                Select an artifact to preview it.
              </Text>
            </View>
          )}
        </>
      ) : (
        <SummaryTab state={state} activeTab={activeTab} />
      )}
    </View>
  );
}

export function RightDock(props: RightDockProps) {
  const { width } = useWindowDimensions();
  const inline = isArtifactSidebarInline(width);
  const open = useUiStore((s) => s.artifactSidebarOpen);
  const dockWidth = useUiStore((s) => s.artifactSidebarWidth);
  const setDockWidth = useUiStore((s) => s.setArtifactSidebarWidth);
  const close = useUiStore((s) => s.closeArtifactPreview);
  const { animatedStyle, resizeGesture, handleCursorStyle } = useResizableSidebarWidth({
    width: dockWidth,
    setWidth: setDockWidth,
    clampWidth: clampArtifactSidebarWidth,
    edge: "left",
    enabled: inline,
  });

  if (!open || props.state.tabs.length === 0) return null;
  if (inline) {
    return (
      <Animated.View style={[{ flexShrink: 0, alignSelf: "stretch" }, animatedStyle]}>
        <View style={{ flex: 1, position: "relative" }}>
          <SidebarResizeHandle edge="left" gesture={resizeGesture} cursorStyle={handleCursorStyle} />
          <RightDockContent {...props} />
        </View>
      </Animated.View>
    );
  }
  return (
    <Modal visible transparent animationType="fade" onRequestClose={close}>
      <View style={{ flex: 1, flexDirection: "row", backgroundColor: "rgba(0,0,0,0.55)" }}>
        <Pressable accessibilityLabel="Close right dock" style={{ flex: 1 }} onPress={close} />
        <View style={{ width: Math.min(width * 0.92, ARTIFACT_SIDEBAR_WIDTH + 40), maxWidth: width, flex: 1 }}>
          <RightDockContent {...props} />
        </View>
      </View>
    </Modal>
  );
}
