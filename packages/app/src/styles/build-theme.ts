import { Platform } from "react-native";

import {
  baseFontSize,
  DEFAULT_MONO_FONT_STACK,
  DEFAULT_MONO_FONT_NATIVE,
  DEFAULT_UI_FONT_NATIVE,
  DEFAULT_UI_FONT_STACK,
  hairlineWidth,
  layout,
  motion,
  radius,
  spacing,
} from "./tokens";
import { deriveDensityTokens, type DensityTokens, type UiDensity } from "./density";
import { darkColors } from "./themes/dark";
import { lightColors } from "./themes/light";

export type ColorScheme = "light" | "dark";

export interface ThemeColorSet {
  surface0: string;
  surface1: string;
  surface2: string;
  surface3: string;
  surfaceSidebar: string;
  surfaceSidebarHover: string;
  surfaceWorkspace: string;
  foreground: string;
  foregroundMuted: string;
  border: string;
  borderAccent: string;
  accent: string;
  accentBright: string;
  accentForeground: string;
  destructive: string;
  destructiveForeground: string;
  diffAddition: string;
  diffDeletion: string;
  diffAddTint: string;
  diffRemoveTint: string;
  popupSurface: string;
  popupBorder: string;
  contentSeam: string;
  shadow: string;
}

export interface AppearanceInput {
  uiFontFamily: string;
  monoFontFamily: string;
  uiFontSize: number;
  codeFontSize: number;
  density: UiDensity;
}

export interface AppTheme {
  colorScheme: ColorScheme;
  colors: ThemeColorSet;
  spacing: typeof spacing;
  radius: typeof radius;
  fontSize: {
    xs: number;
    sm: number;
    base: number;
    lg: number;
    code: number;
  };
  lineHeight: {
    diff: number;
  };
  fontFamily: {
    ui: string;
    mono: string;
  };
  layout: typeof layout;
  density: DensityTokens;
  hairlineWidth: typeof hairlineWidth;
  motion: typeof motion;
}

const BASE_UI_REFERENCE = baseFontSize.base;

function scaleFontSize(uiSize: number, codeSize: number): AppTheme["fontSize"] {
  const ratio = uiSize / BASE_UI_REFERENCE;
  return {
    xs: Math.round(baseFontSize.xs * ratio),
    sm: Math.round(baseFontSize.sm * ratio),
    base: Math.round(baseFontSize.base * ratio),
    lg: Math.round(baseFontSize.lg * ratio),
    code: codeSize,
  };
}

function defaultUiFontFamily(): string {
  if (Platform.OS === "web") {
    return DEFAULT_UI_FONT_STACK;
  }
  return DEFAULT_UI_FONT_NATIVE;
}

function defaultMonoFontFamily(): string {
  if (Platform.OS === "web") {
    return DEFAULT_MONO_FONT_STACK;
  }
  return DEFAULT_MONO_FONT_NATIVE;
}

export function buildTheme(colorScheme: ColorScheme, appearance: AppearanceInput): AppTheme {
  const ui = appearance.uiFontFamily.trim() || defaultUiFontFamily();
  const mono = appearance.monoFontFamily.trim() || defaultMonoFontFamily();
  const fontSize = scaleFontSize(appearance.uiFontSize, appearance.codeFontSize);

  return {
    colorScheme,
    colors: colorScheme === "light" ? lightColors : darkColors,
    spacing,
    radius,
    fontSize,
    lineHeight: {
      diff: Math.round(appearance.codeFontSize * 1.5),
    },
    fontFamily: { ui, mono },
    layout,
    density: deriveDensityTokens(appearance.density),
    hairlineWidth,
    motion,
  };
}

export const defaultAppearance: AppearanceInput = {
  uiFontFamily: "",
  monoFontFamily: "",
  uiFontSize: 16,
  codeFontSize: 13,
  density: "comfortable",
};

export const defaultDarkTheme = buildTheme("dark", defaultAppearance);
