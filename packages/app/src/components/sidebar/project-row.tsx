import {
  ChevronRight,
  Folder,
  FolderPlus,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react-native";
import { useState, type ReactNode } from "react";
import { Platform, Pressable, Text, TextInput, View } from "react-native";

import type { ProjectTreeNode } from "@/lib/project-tree";
import { useSidebarRowStyles } from "@/components/sidebar/sidebar-row-styles";
import { CompactIconButton } from "@/components/ui/surface-controls";
import { useTheme } from "@/styles/use-theme";

interface ProjectRowProps {
  node: ProjectTreeNode;
  expanded: boolean;
  selected: boolean;
  compact: boolean;
  canAttachRoot: boolean;
  onToggle: () => void;
  onNewThread?: () => void;
  onRename?: (name: string) => Promise<void>;
  onDelete?: () => Promise<void>;
  onAttachRoot?: () => Promise<void>;
  children: ReactNode;
}

export function ProjectRow({
  node,
  expanded,
  selected,
  compact,
  canAttachRoot,
  onToggle,
  onNewThread,
  onRename,
  onDelete,
  onAttachRoot,
  children,
}: ProjectRowProps) {
  const theme = useTheme();
  const styles = useSidebarRowStyles();
  const [hovered, setHovered] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(node.name);
  const showActions = hovered || compact || Platform.OS !== "web" || renaming;

  const submitRename = async () => {
    const next = name.trim();
    setRenaming(false);
    if (!next || next === node.name || !onRename) {
      setName(node.name);
      return;
    }
    await onRename(next);
  };

  return (
    <View>
      <View
        style={[styles.row, selected ? styles.active : null]}
        onPointerEnter={() => setHovered(true)}
        onPointerLeave={() => setHovered(false)}
      >
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`${node.name} project`}
          accessibilityState={{ expanded, selected }}
          onPress={onToggle}
          style={({ pressed }) => [
            {
              flex: 1,
              flexDirection: "row",
              alignItems: "center",
              gap: theme.spacing[2],
              minWidth: 0,
            },
            pressed ? styles.pressed : null,
          ]}
        >
          <View style={{ transform: [{ rotate: expanded ? "90deg" : "0deg" }] }}>
            <ChevronRight color={theme.colors.foregroundMuted} size={13} strokeWidth={1.8} />
          </View>
          <Folder color={theme.colors.foregroundMuted} size={14} strokeWidth={1.7} />
          {renaming ? (
            <TextInput
              autoFocus
              value={name}
              onChangeText={setName}
              onBlur={() => void submitRename()}
              onSubmitEditing={() => void submitRename()}
              selectTextOnFocus
              style={{
                flex: 1,
                minWidth: 0,
                paddingVertical: 0,
                color: theme.colors.foreground,
                fontSize: theme.fontSize.xs,
              }}
            />
          ) : (
            <Text numberOfLines={1} style={[styles.label, { flex: 1 }]}>
              {node.name}
            </Text>
          )}
          {!showActions ? (
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
              {node.conversations.length || ""}
            </Text>
          ) : null}
        </Pressable>
        {showActions ? (
          <View style={{ flexDirection: "row", alignItems: "center" }}>
            {canAttachRoot && onAttachRoot ? (
              <CompactIconButton
                icon={FolderPlus}
                label={`Add shared root to ${node.name}`}
                size={13}
                onPress={() => void onAttachRoot()}
              />
            ) : null}
            {onRename ? (
              <CompactIconButton
                icon={Pencil}
                label={`Rename ${node.name}`}
                size={12}
                onPress={() => setRenaming(true)}
              />
            ) : null}
            {onDelete ? (
              <CompactIconButton
                icon={Trash2}
                label={`Delete ${node.name}`}
                size={12}
                onPress={() => void onDelete()}
              />
            ) : null}
            {onNewThread ? (
              <CompactIconButton
                icon={Plus}
                label={`New thread in ${node.name}`}
                size={14}
                onPress={() => onNewThread()}
              />
            ) : null}
          </View>
        ) : null}
      </View>
      {expanded ? <View style={{ gap: 2, paddingTop: 2 }}>{children}</View> : null}
    </View>
  );
}
