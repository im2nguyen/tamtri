/** Shared layout tokens — color-agnostic. */

export const spacing = {
  1: 4,
  2: 8,
  3: 12,
  4: 16,
  5: 20,
  6: 24,
  8: 32,
} as const;

export const radius = {
  sm: 4,
  md: 6,
  lg: 8,
  xl: 12,
  contentCard: 14.4,
  userBubble: 12.8,
  full: 9999,
} as const;

export const baseFontSize = {
  xs: 12,
  sm: 14,
  base: 16,
  lg: 18,
} as const;

export const layout = {
  sidebarWidth: 300,
  sidebarMinWidth: 208,
  sidebarMaxWidth: 420,
  maxContentWidth: 820,
  mainMinWidth: 640,
  headerHeight: 46,
  composerMinHeight: 72,
} as const;

export const hairlineWidth = 1;

export const motion = {
  control: 150,
  disclosure: 220,
  pane: 300,
} as const;

/** Primary UI typeface — https://fonts.google.com/specimen/Geist */
export const DEFAULT_UI_FONT = "Geist";

export const DEFAULT_UI_FONT_STACK = `"${DEFAULT_UI_FONT}", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`;

/** Loaded via @expo-google-fonts/geist on iOS/Android. */
export const DEFAULT_UI_FONT_NATIVE = "Geist_400Regular";

/** Primary code typeface. Loaded alongside Geist on every surface. */
export const DEFAULT_MONO_FONT_NATIVE = "GeistMono_400Regular";

export const GEIST_GOOGLE_FONTS_URL =
  "https://fonts.googleapis.com/css2?family=Geist:wght@100..900&family=Geist+Mono:wght@100..900&display=swap";

export const DEFAULT_MONO_FONT_STACK =
  '"Geist Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace';

export const MIN_UI_FONT_SIZE = 12;
export const MAX_UI_FONT_SIZE = 22;
export const MIN_CODE_FONT_SIZE = 10;
export const MAX_CODE_FONT_SIZE = 20;
