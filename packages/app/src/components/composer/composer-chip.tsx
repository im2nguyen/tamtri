import { ChevronDown } from "lucide-react-native";
import { Modal, Pressable, Text, View, type StyleProp, type ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";

interface ComposerChipProps {
  label: string;
  onPress?: () => void;
  disabled?: boolean;
  leading?: React.ReactNode;
  testID?: string;
  style?: StyleProp<ViewStyle>;
}

export function ComposerChip({
  label,
  onPress,
  disabled,
  leading,
  testID,
  style,
}: ComposerChipProps) {
  const theme = useTheme();
  const interactive = Boolean(onPress) && !disabled;

  return (
    <Pressable
      testID={testID}
      onPress={onPress}
      disabled={!interactive}
      style={({ pressed }) => [
        {
          height: 28,
          maxWidth: 160,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[1],
          paddingHorizontal: theme.spacing[2],
          borderRadius: theme.radius.full,
          backgroundColor: pressed && interactive ? theme.colors.surface3 : theme.colors.surface2,
          opacity: disabled ? 0.55 : 1,
        },
        style,
      ]}
    >
      {leading}
      <Text
        numberOfLines={1}
        style={{
          color: theme.colors.foregroundMuted,
          fontSize: theme.fontSize.xs,
          fontWeight: "500",
          flexShrink: 1,
        }}
      >
        {label}
      </Text>
      {interactive ? <ChevronDown color={theme.colors.foregroundMuted} size={12} /> : null}
    </Pressable>
  );
}

interface ComposerInfoSheetProps {
  visible: boolean;
  title: string;
  body: string;
  primaryLabel?: string;
  onPrimary?: () => void;
  onClose: () => void;
}

export function ComposerInfoSheet({
  visible,
  title,
  body,
  primaryLabel,
  onPrimary,
  onClose,
}: ComposerInfoSheetProps) {
  const theme = useTheme();
  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <Pressable
        onPress={onClose}
        style={{
          flex: 1,
          backgroundColor: "rgba(0,0,0,0.45)",
          justifyContent: "center",
          padding: theme.spacing[4],
        }}
      >
        <Pressable
          onPress={(event) => event.stopPropagation()}
          style={{
            alignSelf: "center",
            width: "100%",
            maxWidth: 420,
            backgroundColor: theme.colors.surface1,
            borderRadius: theme.radius.xl,
            borderWidth: 1,
            borderColor: theme.colors.border,
            padding: theme.spacing[4],
            gap: theme.spacing[3],
          }}
        >
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "700" }}>
            {title}
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
            {body}
          </Text>
          <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[2] }}>
            <Pressable
              onPress={onClose}
              style={({ pressed }) => ({
                paddingHorizontal: theme.spacing[3],
                paddingVertical: theme.spacing[2],
                borderRadius: theme.radius.lg,
                backgroundColor: pressed ? theme.colors.surface2 : "transparent",
              })}
            >
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>Close</Text>
            </Pressable>
            {primaryLabel && onPrimary ? (
              <Pressable
                onPress={() => {
                  onPrimary();
                  onClose();
                }}
                style={({ pressed }) => ({
                  paddingHorizontal: theme.spacing[3],
                  paddingVertical: theme.spacing[2],
                  borderRadius: theme.radius.lg,
                  backgroundColor: pressed ? theme.colors.accentBright : theme.colors.accent,
                })}
              >
                <Text style={{ color: theme.colors.accentForeground, fontSize: theme.fontSize.sm, fontWeight: "600" }}>
                  {primaryLabel}
                </Text>
              </Pressable>
            ) : null}
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}

export function shortModelLabel(modelId: string): string {
  const slash = modelId.lastIndexOf("/");
  if (slash === -1) return modelId;
  return modelId.slice(slash + 1);
}
