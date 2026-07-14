import { ChevronRight } from "lucide-react-native";
import { useState, type ReactNode } from "react";
import { Platform, Pressable, Text, View, type ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";

export interface DisclosureProps {
  title: ReactNode;
  children: ReactNode;
  defaultOpen?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  accessibilityLabel?: string;
}

export function Disclosure({
  title,
  children,
  defaultOpen = false,
  open: controlledOpen,
  onOpenChange,
  accessibilityLabel,
}: DisclosureProps) {
  const theme = useTheme();
  const [uncontrolledOpen, setUncontrolledOpen] = useState(defaultOpen);
  const open = controlledOpen ?? uncontrolledOpen;
  const setOpen = (next: boolean) => {
    if (controlledOpen === undefined) setUncontrolledOpen(next);
    onOpenChange?.(next);
  };
  const transition =
    Platform.OS === "web"
      ? ({
          transitionDuration: "var(--tamtri-motion-disclosure, 220ms)",
          transitionProperty: "transform",
          transitionTimingFunction: "ease",
        } as unknown as ViewStyle)
      : undefined;

  return (
    <View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={accessibilityLabel ?? (typeof title === "string" ? title : undefined)}
        accessibilityState={{ expanded: open }}
        onPress={() => setOpen(!open)}
        style={({ pressed }) => ({
          minHeight: theme.density.rowHeight,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.density.rowGap,
          borderRadius: theme.radius.md,
          opacity: pressed ? 0.7 : 1,
        })}
      >
        <View style={[{ transform: [{ rotate: open ? "90deg" : "0deg" }] }, transition]}>
          <ChevronRight color={theme.colors.foregroundMuted} size={14} strokeWidth={1.8} />
        </View>
        {typeof title === "string" ? (
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>{title}</Text>
        ) : (
          title
        )}
      </Pressable>
      {open ? <View>{children}</View> : null}
    </View>
  );
}
