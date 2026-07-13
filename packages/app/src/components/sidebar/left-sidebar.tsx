import { useRouter, usePathname } from "expo-router";
import { PanelLeft, Plus, Settings2 } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ActivityIndicator,
  Platform,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  useWindowDimensions,
  View,
} from "react-native";
import Animated from "react-native-reanimated";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { SidebarResizeHandle } from "@/components/layout/sidebar-resize-handle";
import { ProjectSidebar } from "@/components/sidebar/project-sidebar";
import { MoveThreadSheet } from "@/components/sidebar/move-thread-sheet";
import { SearchResultRow } from "@/components/sidebar/search-result-row";
import { SearchInput } from "@/components/ui/search-input";
import { CompactIconButton } from "@/components/ui/surface-controls";
import { clampLeftSidebarWidth, isCompact } from "@/constants/layout";
import { useConversationList } from "@/hooks/use-conversations";
import { useProjects } from "@/hooks/use-projects";
import { useResizableSidebarWidth } from "@/hooks/use-resizable-sidebar-width";
import { useSearch } from "@/hooks/use-search";
import { useVaultIssues } from "@/hooks/use-vault-issues";
import { buildProjectTree, UNFILED_PROJECT_ID } from "@/lib/project-tree";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface LeftSidebarProps {
  onClose?: () => void;
}

export function LeftSidebar({ onClose }: LeftSidebarProps) {
  const theme = useTheme();
  const router = useRouter();
  const pathname = usePathname();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const { conversations, loading: conversationsLoading } = useConversationList();
  const {
    projects,
    projectsSupported,
    loading: projectsLoading,
    error: projectsError,
    createProject,
    renameProject,
    deleteProject,
    attachFilesystemRoot,
    canAttachFilesystemRoot,
    moveConversationToProject,
    removeProjectRoot,
  } = useProjects();
  const { hits, loading: searchLoading, scopeMessage, search } = useSearch();
  const { issues } = useVaultIssues();
  const [query, setQuery] = useState("");
  const [creatingProject, setCreatingProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [moveTarget, setMoveTarget] = useState<{
    conversationId: string;
    currentProjectId: string;
  } | null>(null);
  const sidebarWidth = useUiStore((s) => s.sidebarWidth);
  const setSidebarWidth = useUiStore((s) => s.setSidebarWidth);
  const beginProjectDraft = useUiStore((s) => s.beginProjectDraft);
  const setSelectedProject = useUiStore((s) => s.setSelectedProject);
  const clearProjectDraft = useUiStore((s) => s.clearProjectDraft);
  const { animatedStyle, resizeGesture, handleCursorStyle } = useResizableSidebarWidth({
    width: sidebarWidth,
    setWidth: setSidebarWidth,
    clampWidth: clampLeftSidebarWidth,
    edge: "right",
    enabled: !compact,
  });

  useEffect(() => {
    const handle = setTimeout(() => {
      void search(query);
    }, 250);
    return () => clearTimeout(handle);
  }, [query, search]);

  const isSearching = query.trim().length > 0;
  const tree = useMemo(
    () => buildProjectTree(projects, conversations),
    [conversations, projects],
  );
  const activeConversationId = pathname.startsWith("/conversation/")
    ? pathname.slice("/conversation/".length)
    : undefined;
  const loading = conversationsLoading || projectsLoading;

  const submitProject = useCallback(async () => {
    const name = newProjectName.trim();
    if (!name) return;
    const project = await createProject(name);
    setCreatingProject(false);
    setNewProjectName("");
    beginProjectDraft(project.id);
    router.push("/");
    onClose?.();
  }, [beginProjectDraft, createProject, newProjectName, onClose, router]);

  const sidebarBody = (
    <View
      style={{
        flex: 1,
        width: compact ? "100%" : undefined,
        position: "relative",
        backgroundColor: theme.colors.surfaceSidebar,
        paddingTop: compact ? 12 : 38,
        ...(Platform.OS === "web"
          ? ({ backdropFilter: "blur(28px) saturate(145%)" } as object)
          : {}),
      }}
    >
      <TitlebarDragRegion />
      <View style={{ paddingHorizontal: theme.spacing[3], paddingBottom: theme.spacing[3] }}>
        <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "600" }}>tamtri</Text>
          {issues.length > 0 ? (
            <View
              style={{
                paddingHorizontal: 8,
                paddingVertical: 2,
                borderRadius: theme.radius.full,
                backgroundColor: theme.colors.destructive,
              }}
            >
              <Text style={{ color: theme.colors.destructiveForeground, fontSize: theme.fontSize.xs, fontWeight: "700" }}>
                {issues.length} vault issue{issues.length === 1 ? "" : "s"}
              </Text>
            </View>
          ) : null}
          {compact ? (
            <Pressable onPress={onClose} hitSlop={8}>
              <PanelLeft color={theme.colors.foregroundMuted} size={18} />
            </Pressable>
          ) : null}
        </View>
      </View>

      <View style={{ paddingHorizontal: theme.spacing[2], gap: theme.spacing[2], marginBottom: theme.spacing[2] }}>
        <SearchInput
          value={query}
          onChangeText={setQuery}
          onClear={() => setQuery("")}
          placeholder="Search"
          containerStyle={{ backgroundColor: "transparent" }}
        />
        <View style={{ flexDirection: "row", alignItems: "center", paddingHorizontal: theme.spacing[2] }}>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, flex: 1 }}>
            Projects
          </Text>
          {projectsSupported ? (
            <CompactIconButton
              icon={Plus}
              label="New project"
              size={15}
              onPress={() => setCreatingProject(true)}
            />
          ) : null}
        </View>
        {creatingProject ? (
          <TextInput
            autoFocus
            value={newProjectName}
            onChangeText={setNewProjectName}
            onSubmitEditing={() => void submitProject()}
            onBlur={() => {
              if (!newProjectName.trim()) setCreatingProject(false);
            }}
            placeholder="Project name"
            placeholderTextColor={theme.colors.foregroundMuted}
            style={{
              minHeight: 32,
              marginHorizontal: theme.spacing[1],
              paddingHorizontal: theme.spacing[2],
              borderRadius: theme.radius.md,
              borderWidth: theme.hairlineWidth,
              borderColor: theme.colors.borderAccent,
              color: theme.colors.foreground,
              fontSize: theme.fontSize.xs,
            }}
          />
        ) : null}
      </View>

      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{ paddingHorizontal: theme.spacing[2], paddingBottom: 24 }}
      >
        {projectsError ? (
          <Text
            accessibilityRole="alert"
            style={{
              color: theme.colors.foregroundMuted,
              fontSize: theme.fontSize.sm,
              lineHeight: 20,
              padding: theme.spacing[3],
            }}
          >
            {projectsError}
          </Text>
        ) : loading || (isSearching && searchLoading) ? (
          <ActivityIndicator color={theme.colors.accentBright} style={{ marginTop: 24 }} />
        ) : isSearching ? (
          hits.length === 0 ? (
            <Text style={{ color: theme.colors.foregroundMuted, textAlign: "center", marginTop: 24, fontSize: theme.fontSize.sm }}>
              No matches. {scopeMessage}
            </Text>
          ) : (
            hits.map((hit) => (
              <SearchResultRow
                key={`${hit.conversation_id}-${hit.match_field}`}
                hit={hit}
                selected={pathname === `/conversation/${hit.conversation_id}`}
                onPress={() => {
                  const conversation = conversations.find(
                    (row) => row.id === hit.conversation_id,
                  );
                  if (
                    conversation?.project_id &&
                    conversation.project_id !== UNFILED_PROJECT_ID
                  ) {
                    setSelectedProject(conversation.project_id);
                  }
                  clearProjectDraft();
                  router.push(`/conversation/${hit.conversation_id}`);
                  onClose?.();
                }}
              />
            ))
          )
        ) : (
          <ProjectSidebar
            nodes={tree}
            activeConversationId={activeConversationId}
            compact={compact}
            canAttachRoot={canAttachFilesystemRoot}
            onClose={onClose}
            onRename={async (projectId, name) => {
              await renameProject(projectId, name);
            }}
            onDelete={async (projectId) => {
              await deleteProject(projectId);
            }}
            onAttachRoot={async (projectId) => {
              await attachFilesystemRoot(projectId);
            }}
            onMoveConversation={(conversationId, currentProjectId) => {
              setMoveTarget({ conversationId, currentProjectId });
            }}
            onRemoveRoot={async (projectId, rootId) => {
              await removeProjectRoot(projectId, rootId);
            }}
          />
        )}
      </ScrollView>

      <MoveThreadSheet
        visible={moveTarget !== null}
        projects={projects}
        currentProjectId={moveTarget?.currentProjectId}
        onClose={() => setMoveTarget(null)}
        onSelect={async (projectId) => {
          if (!moveTarget) return;
          await moveConversationToProject(moveTarget.conversationId, projectId);
          if (projectId !== UNFILED_PROJECT_ID) {
            setSelectedProject(projectId);
          }
          setMoveTarget(null);
          onClose?.();
        }}
      />

      <View style={{ padding: theme.spacing[2] }}>
        <Pressable
          onPress={() => {
            router.push("/settings/general");
            onClose?.();
          }}
          style={({ pressed }) => ({
            flexDirection: "row",
            alignItems: "center",
            gap: theme.spacing[3],
            minHeight: theme.density.rowHeight,
            paddingHorizontal: theme.spacing[2],
            paddingVertical: theme.density.rowPaddingY,
            borderRadius: theme.radius.md,
            backgroundColor:
              pathname.startsWith("/settings") || pathname === "/health"
                ? theme.colors.surface2
                : pressed
                  ? theme.colors.surfaceSidebarHover
                  : "transparent",
          })}
        >
          <Settings2
            color={pathname.startsWith("/settings") || pathname === "/health" ? theme.colors.accentBright : theme.colors.foregroundMuted}
            size={16}
          />
          <Text
            style={{
              color: pathname.startsWith("/settings") || pathname === "/health" ? theme.colors.foreground : theme.colors.foregroundMuted,
              fontSize: theme.fontSize.sm,
              fontWeight: pathname.startsWith("/settings") || pathname === "/health" ? "600" : "400",
              flex: 1,
            }}
          >
            Settings
          </Text>
          {issues.length > 0 ? (
            <View
              style={{
                minWidth: 20,
                height: 20,
                borderRadius: theme.radius.full,
                backgroundColor: theme.colors.destructive,
                alignItems: "center",
                justifyContent: "center",
                paddingHorizontal: 6,
              }}
            >
              <Text style={{ color: theme.colors.destructiveForeground, fontSize: theme.fontSize.xs, fontWeight: "700" }}>
                {issues.length}
              </Text>
            </View>
          ) : null}
        </Pressable>
      </View>

      {!compact ? (
        <SidebarResizeHandle edge="right" gesture={resizeGesture} cursorStyle={handleCursorStyle} />
      ) : null}
    </View>
  );

  if (compact) {
    return sidebarBody;
  }

  return (
    <Animated.View style={[{ flexShrink: 0, alignSelf: "stretch" }, animatedStyle]}>
      {sidebarBody}
    </Animated.View>
  );
}
