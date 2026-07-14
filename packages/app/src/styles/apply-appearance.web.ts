import type { AppTheme } from "./build-theme";

const APPEARANCE_STYLE_ID = "tamtri-appearance";

const CSS_VAR_MAP: Array<[keyof AppTheme["colors"], string]> = [
  ["surface0", "--tamtri-surface-0"],
  ["surface1", "--tamtri-surface-1"],
  ["surface2", "--tamtri-surface-2"],
  ["surface3", "--tamtri-surface-3"],
  ["surfaceSidebar", "--tamtri-surface-sidebar"],
  ["surfaceSidebarHover", "--tamtri-surface-sidebar-hover"],
  ["surfaceWorkspace", "--tamtri-surface-workspace"],
  ["foreground", "--tamtri-foreground"],
  ["foregroundMuted", "--tamtri-foreground-muted"],
  ["border", "--tamtri-border"],
  ["borderAccent", "--tamtri-border-accent"],
  ["accent", "--tamtri-accent"],
  ["accentBright", "--tamtri-accent-bright"],
  ["accentForeground", "--tamtri-accent-foreground"],
  ["destructive", "--tamtri-destructive"],
  ["destructiveForeground", "--tamtri-destructive-foreground"],
  ["diffAddition", "--tamtri-diff-addition"],
  ["diffDeletion", "--tamtri-diff-deletion"],
  ["diffAddTint", "--tamtri-diff-add-tint"],
  ["diffRemoveTint", "--tamtri-diff-remove-tint"],
  ["popupSurface", "--tamtri-popup-surface"],
  ["popupBorder", "--tamtri-popup-border"],
  ["contentSeam", "--tamtri-content-seam"],
  ["shadow", "--tamtri-shadow"],
];

function buildAppearanceCss(theme: AppTheme): string {
  const colorVars = CSS_VAR_MAP.map(([key, cssVar]) => `${cssVar}: ${theme.colors[key]};`).join("\n  ");

  return `
:root {
  ${colorVars}
  --tamtri-font-ui: ${theme.fontFamily.ui};
  --tamtri-font-mono: ${theme.fontFamily.mono};
  --tamtri-font-size-base: ${theme.fontSize.base}px;
  --tamtri-font-size-code: ${theme.fontSize.code}px;
  --tamtri-density-scale: ${theme.density.scale};
  --tamtri-row-height: ${theme.density.rowHeight}px;
  --tamtri-row-padding-y: ${theme.density.rowPaddingY}px;
  --tamtri-row-gap: ${theme.density.rowGap}px;
  --tamtri-chat-gutter: ${theme.density.chatGutter}px;
  --tamtri-chat-gutter-wide: ${theme.density.chatGutterWide}px;
  --tamtri-composer-padding-x: ${theme.density.composerPaddingX}px;
  --tamtri-composer-padding-top: ${theme.density.composerPaddingTop}px;
  --tamtri-composer-padding-bottom: ${theme.density.composerPaddingBottom}px;
  --tamtri-composer-footer-padding: ${theme.density.composerFooterPadding}px;
  --tamtri-settings-row-padding-y: ${theme.density.settingsRowPaddingY}px;
  --tamtri-motion-control: ${theme.motion.control}ms;
  --tamtri-motion-disclosure: ${theme.motion.disclosure}ms;
  --tamtri-motion-pane: ${theme.motion.pane}ms;
  --tamtri-sidebar-width: ${theme.layout.sidebarWidth}px;
  --tamtri-sidebar-min-width: ${theme.layout.sidebarMinWidth}px;
  --tamtri-main-min-width: ${theme.layout.mainMinWidth}px;
  --tamtri-header-height: ${theme.layout.headerHeight}px;
  --tamtri-content-card-radius: ${theme.radius.contentCard}px;
  --tamtri-user-bubble-radius: ${theme.radius.userBubble}px;
  --tamtri-hairline: ${theme.hairlineWidth}px;
}
#root {
  font-family: var(--tamtri-font-ui);
  font-size: var(--tamtri-font-size-base);
}
#root *:not([data-tamtri-mono="true"]) {
  font-family: inherit !important;
}
#root [data-tamtri-mono="true"] {
  font-family: var(--tamtri-font-mono) !important;
}
#root *[style*="ui-monospace"],
#root *[style*="SFMono"],
#root *[style*="Consolas"],
#root *[style*="Menlo"],
#root *[style*="monospace"] {
  font-family: var(--tamtri-font-mono) !important;
}
@media (prefers-reduced-motion: reduce) {
  :root {
    --tamtri-motion-control: 0ms;
    --tamtri-motion-disclosure: 0ms;
    --tamtri-motion-pane: 0ms;
  }
}
`.trim();
}

function ensureAppearanceStyleElement(): HTMLStyleElement {
  let style = document.getElementById(APPEARANCE_STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement("style");
    style.id = APPEARANCE_STYLE_ID;
    document.head.appendChild(style);
  }
  return style;
}

export function applyAppearanceCssVars(theme: AppTheme): void {
  if (typeof document === "undefined") return;

  document.documentElement.dataset.colorScheme = theme.colorScheme;
  ensureAppearanceStyleElement().textContent = buildAppearanceCss(theme);
}

export function applyRootUiFont(_theme: AppTheme): void {
  // Fonts are applied via #root rules in the tamtri-appearance style tag.
}
