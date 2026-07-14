import type { LucideIcon } from "lucide-react-native";
import type { ReactNode } from "react";
import {
  Pressable,
  Text,
  View,
  type PressableProps,
  type StyleProp,
  type ViewStyle,
} from "react-native";

import { useTheme } from "@/styles/use-theme";

interface SurfaceChipProps extends Omit<PressableProps, "children" | "style"> {
  label: string;
  icon?: ReactNode;
  selected?: boolean;
  style?: StyleProp<ViewStyle>;
}

export function SurfaceChip({ label, icon, selected, style, ...props }: SurfaceChipProps) {
  const theme = useTheme();
  return (
    <Pressable
      {...props}
      accessibilityRole={props.accessibilityRole ?? "button"}
      accessibilityState={{ ...props.accessibilityState, selected }}
      style={({ pressed }) => [
        {
          minHeight: 28,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[1],
          paddingHorizontal: theme.spacing[2],
          borderRadius: theme.radius.lg,
          backgroundColor: selected
            ? theme.colors.surface2
            : pressed
              ? theme.colors.surfaceSidebarHover
              : "transparent",
          opacity: pressed ? 0.78 : 1,
        },
        style,
      ]}
    >
      {icon}
      <Text
        numberOfLines={1}
        style={{
          color: selected ? theme.colors.foreground : theme.colors.foregroundMuted,
          fontSize: theme.fontSize.xs,
        }}
      >
        {label}
      </Text>
    </Pressable>
  );
}

interface CompactIconButtonProps extends Omit<PressableProps, "children" | "style"> {
  icon: LucideIcon;
  label: string;
  size?: number;
  tone?: "plain" | "outline";
  style?: StyleProp<ViewStyle>;
}

export function CompactIconButton({
  icon: Icon,
  label,
  size = 14,
  tone = "plain",
  style,
  ...props
}: CompactIconButtonProps) {
  const theme = useTheme();
  return (
    <Pressable
      {...props}
      accessibilityRole="button"
      accessibilityLabel={label}
      hitSlop={4}
      style={({ pressed }) => [
        {
          width: 28,
          height: 28,
          alignItems: "center",
          justifyContent: "center",
          borderRadius: theme.radius.lg,
          borderWidth: tone === "outline" ? theme.hairlineWidth : 0,
          borderColor: theme.colors.border,
          backgroundColor: pressed ? theme.colors.surfaceSidebarHover : "transparent",
          opacity: pressed ? 0.72 : 1,
        },
        style,
      ]}
    >
      <View pointerEvents="none">
        <Icon color={theme.colors.foregroundMuted} size={size} strokeWidth={1.8} />
      </View>
    </Pressable>
  );
}
