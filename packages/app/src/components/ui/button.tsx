import { useMemo } from "react";
import { Pressable, Text, type PressableProps, type TextStyle, type ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";

type ButtonVariant = "default" | "secondary" | "ghost" | "destructive";

interface ButtonProps extends PressableProps {
  label: string;
  variant?: ButtonVariant;
  compact?: boolean;
}

export function Button({ label, variant = "default", compact, style, ...props }: ButtonProps) {
  const theme = useTheme();
  const variantStyles = useMemo(
    (): Record<ButtonVariant, { container: ViewStyle; text: TextStyle }> => ({
      default: {
        container: { backgroundColor: theme.colors.accent },
        text: { color: theme.colors.accentForeground },
      },
      secondary: {
        container: {
          backgroundColor: theme.colors.surface2,
          borderWidth: 1,
          borderColor: theme.colors.border,
        },
        text: { color: theme.colors.foreground },
      },
      ghost: {
        container: { backgroundColor: "transparent" },
        text: { color: theme.colors.foregroundMuted },
      },
      destructive: {
        container: { backgroundColor: theme.colors.destructive },
        text: { color: theme.colors.destructiveForeground },
      },
    }),
    [theme],
  );
  const styles = variantStyles[variant];
  return (
    <Pressable
      {...props}
      style={({ pressed }) => [
        {
          borderRadius: theme.radius.lg,
          paddingHorizontal: compact ? theme.spacing[3] : theme.spacing[4],
          paddingVertical: compact ? theme.spacing[2] : theme.spacing[3],
          opacity: pressed ? 0.85 : 1,
          alignItems: "center",
          justifyContent: "center",
        },
        styles.container,
        style as ViewStyle,
      ]}
    >
      <Text style={[{ fontSize: theme.fontSize.sm, fontWeight: "600" }, styles.text]}>{label}</Text>
    </Pressable>
  );
}
