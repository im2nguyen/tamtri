/** Paseo-inspired semantic tokens (dark default). */

export const theme = {
  colors: {
    surface0: "#181B1A",
    surface1: "#1E2120",
    surface2: "#272A29",
    surface3: "#434645",
    surfaceSidebar: "#141716",
    surfaceSidebarHover: "#1c1f1e",
    surfaceWorkspace: "#1E2120",
    foreground: "#fafafa",
    foregroundMuted: "#A1A5A4",
    border: "#252B2A",
    borderAccent: "#2F3534",
    accent: "#20744A",
    accentBright: "#7ccba0",
    accentForeground: "#ffffff",
    destructive: "#c64f43",
    destructiveForeground: "#ffffff",
  },
  spacing: {
    1: 4,
    2: 8,
    3: 12,
    4: 16,
    5: 20,
    6: 24,
    8: 32,
  },
  radius: {
    sm: 4,
    md: 6,
    lg: 8,
    xl: 12,
    full: 9999,
  },
  fontSize: {
    xs: 12,
    sm: 14,
    base: 16,
    lg: 18,
  },
  layout: {
    sidebarWidth: 300,
    sidebarMinWidth: 240,
    sidebarMaxWidth: 420,
    maxContentWidth: 820,
    headerHeight: 48,
    composerMinHeight: 72,
  },
} as const;

export type AppTheme = typeof theme;
