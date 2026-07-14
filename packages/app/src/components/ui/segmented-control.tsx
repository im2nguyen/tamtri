import { Pressable, Text, View, type StyleProp, type ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: React.ReactNode;
}

export interface SegmentedControlProps<T extends string> {
  value: T;
  options: readonly SegmentedOption<T>[];
  onValueChange: (value: T) => void;
  accessibilityLabel: string;
  style?: StyleProp<ViewStyle>;
}

export function SegmentedControl<T extends string>({
  value,
  options,
  onValueChange,
  accessibilityLabel,
  style,
}: SegmentedControlProps<T>) {
  const theme = useTheme();
  return (
    <View
      accessibilityRole="radiogroup"
      accessibilityLabel={accessibilityLabel}
      style={[
        {
          flexDirection: "row",
          alignItems: "center",
          gap: 2,
          padding: 2,
          borderRadius: theme.radius.lg,
          borderWidth: theme.hairlineWidth,
          borderColor: theme.colors.border,
          backgroundColor: theme.colors.surface2,
        },
        style,
      ]}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <Pressable
            key={option.value}
            accessibilityRole="radio"
            accessibilityState={{ checked: selected }}
            onPress={() => onValueChange(option.value)}
            style={({ pressed }) => ({
              minHeight: 28,
              flex: 1,
              flexDirection: "row",
              alignItems: "center",
              justifyContent: "center",
              gap: theme.spacing[1],
              paddingHorizontal: theme.spacing[2],
              borderRadius: theme.radius.md,
              borderWidth: selected ? theme.hairlineWidth : 0,
              borderColor: theme.colors.border,
              backgroundColor: selected
                ? theme.colors.surface1
                : pressed
                  ? theme.colors.surfaceSidebarHover
                  : "transparent",
              opacity: pressed ? 0.82 : 1,
            })}
          >
            {option.icon}
            <Text
              numberOfLines={1}
              style={{
                color: selected ? theme.colors.foreground : theme.colors.foregroundMuted,
                fontSize: theme.fontSize.xs,
                fontWeight: selected ? "500" : "400",
              }}
            >
              {option.label}
            </Text>
          </Pressable>
        );
      })}
    </View>
  );
}
