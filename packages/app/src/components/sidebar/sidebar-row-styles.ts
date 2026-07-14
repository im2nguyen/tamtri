import { useMemo } from "react";
import type { TextStyle, ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";

export function useSidebarRowStyles() {
  const theme = useTheme();
  return useMemo(
    () => ({
      row: {
        minHeight: theme.density.rowHeight,
        flexDirection: "row",
        alignItems: "center",
        gap: theme.density.rowGap,
        paddingHorizontal: theme.spacing[2],
        paddingVertical: theme.density.rowPaddingY,
        borderRadius: theme.radius.md,
      } satisfies ViewStyle,
      active: {
        backgroundColor: theme.colors.surfaceSidebarHover,
      } satisfies ViewStyle,
      pressed: {
        backgroundColor: theme.colors.surfaceSidebarHover,
        opacity: 0.82,
      } satisfies ViewStyle,
      label: {
        flexShrink: 1,
        color: theme.colors.foreground,
        fontSize: theme.fontSize.xs,
        fontWeight: "400",
      } satisfies TextStyle,
      sectionLabel: {
        color: theme.colors.foregroundMuted,
        fontSize: theme.fontSize.xs,
        fontWeight: "400",
        opacity: 0.72,
      } satisfies TextStyle,
    }),
    [theme],
  );
}
