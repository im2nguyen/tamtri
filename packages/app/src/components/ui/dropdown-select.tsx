import { useCallback, useMemo, useRef, useState } from "react";
import {
  Modal,
  Platform,
  Pressable,
  Text,
  View,
  type PressableStateCallbackType,
} from "react-native";

import { useTheme } from "@/styles/use-theme";

export interface DropdownOption<T extends string> {
  value: T;
  label: string;
  leading?: React.ReactNode;
}

interface DropdownSelectProps<T extends string> {
  value: T;
  options: readonly DropdownOption<T>[];
  onChange: (value: T) => void;
  accessibilityLabel: string;
  width?: number;
}

function triggerStyle({ pressed }: PressableStateCallbackType, theme: ReturnType<typeof useTheme>) {
  return {
    flexDirection: "row" as const,
    alignItems: "center" as const,
    gap: theme.spacing[1],
    paddingVertical: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.radius.md,
    borderWidth: 1,
    borderColor: theme.colors.border,
    backgroundColor: theme.colors.surface2,
    opacity: pressed ? 0.85 : 1,
  };
}

export function DropdownSelect<T extends string>({
  value,
  options,
  onChange,
  accessibilityLabel,
  width = 200,
}: DropdownSelectProps<T>) {
  const theme = useTheme();
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<View>(null);
  const selected = options.find((option) => option.value === value) ?? options[0];

  const handleSelect = useCallback(
    (next: T) => {
      onChange(next);
      setOpen(false);
    },
    [onChange],
  );

  const menuStyle = useMemo(
    () => ({
      backgroundColor: theme.colors.surface1,
      borderRadius: theme.radius.lg,
      borderWidth: 1,
      borderColor: theme.colors.border,
      overflow: "hidden" as const,
      minWidth: width,
      ...(Platform.OS === "web"
        ? { boxShadow: "0 8px 24px rgba(0,0,0,0.18)" as const }
        : null),
    }),
    [theme, width],
  );

  return (
    <>
      <Pressable
        ref={triggerRef}
        accessibilityRole="button"
        accessibilityLabel={accessibilityLabel}
        onPress={() => setOpen(true)}
        style={({ pressed }) => triggerStyle({ pressed }, theme)}
      >
        {selected.leading}
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>{selected.label}</Text>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>▾</Text>
      </Pressable>

      <Modal visible={open} transparent animationType="fade" onRequestClose={() => setOpen(false)}>
        <Pressable
          style={{ flex: 1, backgroundColor: "rgba(0,0,0,0.35)", justifyContent: "center", alignItems: "center" }}
          onPress={() => setOpen(false)}
        >
          <Pressable style={menuStyle} onPress={(event) => event.stopPropagation()}>
            {options.map((option, index) => {
              const isSelected = option.value === value;
              return (
                <Pressable
                  key={option.value}
                  accessibilityRole="menuitem"
                  onPress={() => handleSelect(option.value)}
                  style={({ pressed }) => ({
                    flexDirection: "row",
                    alignItems: "center",
                    gap: theme.spacing[2],
                    paddingVertical: theme.spacing[3],
                    paddingHorizontal: theme.spacing[4],
                    backgroundColor: pressed
                      ? theme.colors.surface2
                      : isSelected
                        ? theme.colors.surfaceSidebarHover
                        : "transparent",
                    borderTopWidth: index === 0 ? 0 : 1,
                    borderTopColor: theme.colors.border,
                  })}
                >
                  {option.leading}
                  <Text
                    style={{
                      color: theme.colors.foreground,
                      fontSize: theme.fontSize.sm,
                      fontWeight: isSelected ? "600" : "400",
                    }}
                  >
                    {option.label}
                  </Text>
                </Pressable>
              );
            })}
          </Pressable>
        </Pressable>
      </Modal>
    </>
  );
}
