import { useRouter } from "expo-router";
import { Text, View, Pressable } from "react-native";

import { ProjectRow } from "@/components/sidebar/project-row";
import { ThreadRow } from "@/components/sidebar/thread-row";
import type { ProjectTreeNode } from "@/lib/project-tree";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface ProjectSidebarProps {
  nodes: ProjectTreeNode[];
  activeConversationId?: string;
  compact: boolean;
  canAttachRoot: boolean;
  onClose?: () => void;
  onRename: (projectId: string, name: string) => Promise<void>;
  onDelete: (projectId: string) => Promise<void>;
  onAttachRoot: (projectId: string) => Promise<void>;
  onMoveConversation: (conversationId: string, currentProjectId: string) => void;
  onRemoveRoot: (projectId: string, rootId: string) => Promise<void>;
}

export function ProjectSidebar({
  nodes,
  activeConversationId,
  compact,
  canAttachRoot,
  onClose,
  onRename,
  onDelete,
  onAttachRoot,
  onMoveConversation,
  onRemoveRoot,
}: ProjectSidebarProps) {
  const theme = useTheme();
  const router = useRouter();
  const expandedProjectIds = useUiStore((state) => state.expandedProjectIds);
  const selectedProjectId = useUiStore((state) => state.selectedProjectId);
  const toggleProjectExpanded = useUiStore((state) => state.toggleProjectExpanded);
  const setSelectedProject = useUiStore((state) => state.setSelectedProject);
  const beginProjectDraft = useUiStore((state) => state.beginProjectDraft);
  const clearProjectDraft = useUiStore((state) => state.clearProjectDraft);

  if (nodes.length === 0) {
    return (
      <Text
        style={{
          color: theme.colors.foregroundMuted,
          fontSize: theme.fontSize.xs,
          paddingHorizontal: theme.spacing[2],
          paddingVertical: theme.spacing[4],
        }}
      >
        Create a project to start a thread.
      </Text>
    );
  }

  return (
    <View style={{ gap: theme.spacing[1] }}>
      {nodes.map((node) => {
        const expanded = expandedProjectIds.includes(node.id);
        const selected = selectedProjectId === node.id;
        return (
          <ProjectRow
            key={node.id}
            node={node}
            expanded={expanded}
            selected={selected}
            compact={compact}
            canAttachRoot={canAttachRoot && !node.isUnfiled}
            onToggle={() => {
              toggleProjectExpanded(node.id);
              if (!node.isUnfiled) setSelectedProject(node.id);
            }}
            onNewThread={() => {
              beginProjectDraft(node.id);
              router.push("/");
              onClose?.();
            }}
            onRename={
              node.isUnfiled ? undefined : (name) => onRename(node.id, name)
            }
            onDelete={
              node.isUnfiled ||
              node.conversations.length > 0 ||
              (node.project?.roots.length ?? 0) > 0
                ? undefined
                : () => onDelete(node.id)
            }
            onAttachRoot={
              node.isUnfiled ? undefined : () => onAttachRoot(node.id)
            }
          >
            {(node.project?.roots ?? []).map((root) => (
              <View
                key={root.id}
                style={{
                  flexDirection: "row",
                  alignItems: "center",
                  gap: theme.spacing[2],
                  paddingLeft: theme.spacing[8],
                  paddingRight: theme.spacing[2],
                  paddingVertical: theme.spacing[1],
                }}
              >
                <Text
                  numberOfLines={1}
                  style={{
                    flex: 1,
                    color: theme.colors.foregroundMuted,
                    fontSize: 11,
                    fontFamily: theme.fontFamily.mono,
                  }}
                >
                  {root.name}
                </Text>
                {!node.isUnfiled ? (
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel={`Remove root ${root.name}`}
                    onPress={() => void onRemoveRoot(node.id, root.id)}
                  >
                    <Text style={{ color: theme.colors.destructive, fontSize: 11 }}>Remove</Text>
                  </Pressable>
                ) : null}
              </View>
            ))}
            {node.conversations.map((conversation) => (
              <ThreadRow
                key={conversation.id}
                conversation={conversation}
                selected={activeConversationId === conversation.id}
                onMove={() => onMoveConversation(conversation.id, node.id)}
                onPress={() => {
                  if (!node.isUnfiled) setSelectedProject(node.id);
                  clearProjectDraft();
                  router.push(`/conversation/${conversation.id}`);
                  onClose?.();
                }}
              />
            ))}
            {node.conversations.length === 0 ? (
              <Text
                style={{
                  color: theme.colors.foregroundMuted,
                  fontSize: 11,
                  paddingLeft: theme.spacing[8],
                  paddingVertical: theme.spacing[1],
                }}
              >
                No threads yet
              </Text>
            ) : null}
          </ProjectRow>
        );
      })}
    </View>
  );
}
