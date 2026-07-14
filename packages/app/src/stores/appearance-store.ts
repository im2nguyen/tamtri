import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import { persistedStorage } from "@/lib/persisted-storage";
import { defaultAppearance } from "@/styles/build-theme";
import {
  migrateAppearanceDensityState,
  normalizeUiDensity,
  type UiDensity,
} from "@/styles/density";
import {
  MAX_CODE_FONT_SIZE,
  MAX_UI_FONT_SIZE,
  MIN_CODE_FONT_SIZE,
  MIN_UI_FONT_SIZE,
} from "@/styles/tokens";
import type { SyntaxThemeId } from "@/styles/syntax-themes";

export type ThemeMode = "system" | "light" | "dark";

export interface AppearanceState {
  themeMode: ThemeMode;
  uiFontFamily: string;
  uiFontSize: number;
  monoFontFamily: string;
  codeFontSize: number;
  syntaxTheme: SyntaxThemeId;
  density: UiDensity;
  setThemeMode: (mode: ThemeMode) => void;
  setUiFontFamily: (value: string) => void;
  setUiFontSize: (value: number) => void;
  setMonoFontFamily: (value: string) => void;
  setCodeFontSize: (value: number) => void;
  setSyntaxTheme: (value: SyntaxThemeId) => void;
  setDensity: (value: UiDensity) => void;
}

export interface PersistedAppearance {
  themeMode?: ThemeMode;
  uiFontFamily?: string;
  uiFontSize?: number;
  monoFontFamily?: string;
  codeFontSize?: number;
  syntaxTheme?: SyntaxThemeId;
  density?: UiDensity;
}

export function migrateAppearanceState(persisted: unknown): PersistedAppearance {
  return migrateAppearanceDensityState(persisted) as PersistedAppearance;
}

export function sanitizeFontFamily(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed.length === 0) return "";
  if (trimmed.length > 200) return null;
  if (/[<>"'`\\]/.test(trimmed)) return null;
  return trimmed;
}

export function parseClampedFontSize(
  raw: string,
  bounds: { min: number; max: number },
): number | null {
  if (raw.trim().length === 0) return null;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed)) return null;
  return Math.min(bounds.max, Math.max(bounds.min, parsed));
}

export function clampUiFontSize(value: number): number {
  return Math.min(MAX_UI_FONT_SIZE, Math.max(MIN_UI_FONT_SIZE, value));
}

export function clampCodeFontSize(value: number): number {
  return Math.min(MAX_CODE_FONT_SIZE, Math.max(MIN_CODE_FONT_SIZE, value));
}

export const useAppearanceStore = create<AppearanceState>()(
  persist(
    (set) => ({
      themeMode: "system",
      uiFontFamily: defaultAppearance.uiFontFamily,
      uiFontSize: defaultAppearance.uiFontSize,
      monoFontFamily: defaultAppearance.monoFontFamily,
      codeFontSize: defaultAppearance.codeFontSize,
      syntaxTheme: "one-dark",
      density: defaultAppearance.density,
      setThemeMode: (themeMode) => set({ themeMode }),
      setUiFontFamily: (uiFontFamily) => set({ uiFontFamily }),
      setUiFontSize: (uiFontSize) => set({ uiFontSize: clampUiFontSize(uiFontSize) }),
      setMonoFontFamily: (monoFontFamily) => set({ monoFontFamily }),
      setCodeFontSize: (codeFontSize) => set({ codeFontSize: clampCodeFontSize(codeFontSize) }),
      setSyntaxTheme: (syntaxTheme) => set({ syntaxTheme }),
      setDensity: (density) => set({ density: normalizeUiDensity(density) }),
    }),
    {
      name: "tamtri-appearance",
      version: 2,
      storage: createJSONStorage(() => persistedStorage),
      migrate: (persisted) => migrateAppearanceState(persisted) as AppearanceState,
      partialize: (state) => ({
        themeMode: state.themeMode,
        uiFontFamily: state.uiFontFamily,
        uiFontSize: state.uiFontSize,
        monoFontFamily: state.monoFontFamily,
        codeFontSize: state.codeFontSize,
        syntaxTheme: state.syntaxTheme,
        density: state.density,
      }),
    },
  ),
);

export function selectAppearanceInput(state: AppearanceState) {
  return {
    uiFontFamily: state.uiFontFamily,
    monoFontFamily: state.monoFontFamily,
    uiFontSize: state.uiFontSize,
    codeFontSize: state.codeFontSize,
    density: state.density,
  };
}
