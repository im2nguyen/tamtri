import { FileText, FolderRoot, Paperclip, Plus } from "lucide-react-native";
import { useState } from "react";
import { ActivityIndicator, Modal, Platform, Pressable, Text, View } from "react-native";

import { useTheme } from "@/styles/use-theme";

export interface ComposerMenuItem {
  id: string;
  label: string;
  icon: React.ReactNode;
  onSelect: () => void;
  disabled?: boolean;
}

interface ComposerAttachmentMenuProps {
  disabled?: boolean;
  attaching?: boolean;
  items: ComposerMenuItem[];
}

export function ComposerAttachmentMenu({ disabled, attaching, items }: ComposerAttachmentMenuProps) {
  const theme = useTheme();
  const [open, setOpen] = useState(false);

  const visibleItems = items.filter((item) => !item.disabled);
  if (visibleItems.length === 0) return null;

  const close = () => setOpen(false);

  return (
    <>
      <Pressable
        testID="composer-attach-button"
        onPress={() => setOpen(true)}
        disabled={disabled || attaching}
        hitSlop={6}
        style={({ pressed }) => ({
          width: 28,
          height: 28,
          borderRadius: theme.radius.full,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: pressed ? theme.colors.surface3 : "transparent",
          opacity: disabled ? 0.5 : 1,
        })}
      >
        {attaching ? (
          <ActivityIndicator color={theme.colors.foregroundMuted} size="small" />
        ) : (
          <Plus color={theme.colors.foregroundMuted} size={18} />
        )}
      </Pressable>

      <Modal visible={open} transparent animationType="fade" onRequestClose={close}>
        <Pressable
          onPress={close}
          style={{
            flex: 1,
            backgroundColor: "rgba(0,0,0,0.45)",
            justifyContent: Platform.OS === "web" ? "center" : "flex-end",
            padding: theme.spacing[4],
          }}
        >
          <Pressable
            onPress={(event) => event.stopPropagation()}
            style={{
              alignSelf: Platform.OS === "web" ? "center" : "stretch",
              width: Platform.OS === "web" ? 280 : "100%",
              backgroundColor: theme.colors.surface1,
              borderRadius: theme.radius.xl,
              borderWidth: 1,
              borderColor: theme.colors.border,
              padding: theme.spacing[2],
              gap: theme.spacing[1],
            }}
          >
            <Text
              style={{
                color: theme.colors.foregroundMuted,
                fontSize: theme.fontSize.xs,
                fontWeight: "600",
                paddingHorizontal: theme.spacing[2],
                paddingVertical: theme.spacing[1],
              }}
            >
              ATTACH
            </Text>
            {visibleItems.map((item) => (
              <Pressable
                key={item.id}
                testID={`composer-attach-item-${item.id}`}
                onPress={() => {
                  close();
                  item.onSelect();
                }}
                style={({ pressed }) => ({
                  flexDirection: "row",
                  alignItems: "center",
                  gap: theme.spacing[3],
                  minHeight: 44,
                  paddingHorizontal: theme.spacing[3],
                  paddingVertical: theme.spacing[2],
                  borderRadius: theme.radius.lg,
                  backgroundColor: pressed ? theme.colors.surface2 : "transparent",
                })}
              >
                <View style={{ width: 20, alignItems: "center" }}>{item.icon}</View>
                <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>{item.label}</Text>
              </Pressable>
            ))}
          </Pressable>
        </Pressable>
      </Modal>
    </>
  );
}

export function buildDefaultAttachmentItems(input: {
  onAttachFile?: () => void;
  onAttachRoot?: () => void;
  canAttachRoot?: boolean;
  iconColor: string;
}): ComposerMenuItem[] {
  const items: ComposerMenuItem[] = [];
  if (input.onAttachFile) {
    items.push({
      id: "file",
      label: "Upload file",
      icon: <Paperclip color={input.iconColor} size={16} />,
      onSelect: input.onAttachFile,
    });
  }
  if (input.onAttachRoot) {
    items.push({
      id: "root",
      label: "Attach root",
      icon: <FolderRoot color={input.iconColor} size={16} />,
      onSelect: input.onAttachRoot,
      disabled: !input.canAttachRoot,
    });
  }
  return items;
}

export function AttachmentPill({ name }: { name: string }) {
  const theme = useTheme();
  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[1],
        paddingHorizontal: theme.spacing[2],
        paddingVertical: 4,
        borderRadius: theme.radius.full,
        backgroundColor: theme.colors.surface2,
        borderWidth: 1,
        borderColor: theme.colors.border,
      }}
    >
      <FileText color={theme.colors.foregroundMuted} size={12} />
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }} numberOfLines={1}>
        {name}
      </Text>
    </View>
  );
}
