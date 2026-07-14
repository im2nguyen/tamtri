export type { AppTheme, ColorScheme, AppearanceInput, ThemeColorSet } from "./build-theme";
export { buildTheme, defaultAppearance, defaultDarkTheme } from "./build-theme";
export {
  DEFAULT_UI_DENSITY,
  UI_DENSITIES,
  deriveDensityTokens,
  isUiDensity,
  migrateAppearanceDensityState,
  normalizeUiDensity,
} from "./density";
export type { DensityTokens, UiDensity } from "./density";
export {
  DEFAULT_UI_FONT_STACK,
  DEFAULT_MONO_FONT_STACK,
  spacing,
  radius,
  layout,
  motion,
  hairlineWidth,
  baseFontSize,
  MIN_UI_FONT_SIZE,
  MAX_UI_FONT_SIZE,
  MIN_CODE_FONT_SIZE,
  MAX_CODE_FONT_SIZE,
} from "./tokens";
export type { SyntaxThemeId, SyntaxTokenStyle, SyntaxColors } from "./syntax-themes";
export { SYNTAX_THEME_OPTIONS, syntaxThemeById } from "./syntax-themes";

/** @deprecated Use useTheme() for reactive tokens. Static dark default for legacy imports. */
export { defaultDarkTheme as theme } from "./build-theme";
