import { Pressable, View } from "react-native";

import { useTheme } from "@/styles/use-theme";

interface SwitchProps {
  value: boolean;
  onValueChange: (value: boolean) => void;
  disabled?: boolean;
  accessibilityLabel?: string;
}

export function Switch({ value, onValueChange, disabled, accessibilityLabel }: SwitchProps) {
  const theme = useTheme();
  return (
    <Pressable
      accessibilityRole="switch"
      accessibilityState={{ checked: value, disabled: !!disabled }}
      accessibilityLabel={accessibilityLabel}
      disabled={disabled}
      onPress={() => onValueChange(!value)}
      style={{
        width: 44,
        height: 26,
        borderRadius: theme.radius.full,
        backgroundColor: value ? theme.colors.accent : theme.colors.surface3,
        padding: 2,
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <View
        style={{
          width: 22,
          height: 22,
          borderRadius: theme.radius.full,
          backgroundColor: theme.colors.foreground,
          marginLeft: value ? 18 : 0,
        }}
      />
    </Pressable>
  );
}
