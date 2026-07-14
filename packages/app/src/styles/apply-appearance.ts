import type { AppTheme } from "./build-theme";

/** Apply theme tokens as CSS custom properties on web. No-op import on native. */
export function applyAppearanceCssVars(theme: AppTheme): void {
  // Native stub — web implementation in apply-appearance.web.ts
  void theme;
}

export function applyRootUiFont(theme: AppTheme): void {
  void theme;
}
