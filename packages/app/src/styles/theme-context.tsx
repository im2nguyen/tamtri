import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import { Platform, useColorScheme as useSystemColorScheme } from "react-native";
import { useShallow } from "zustand/react/shallow";

import { selectAppearanceInput, useAppearanceStore, type ThemeMode } from "@/stores/appearance-store";
import { applyAppearanceCssVars, applyRootUiFont } from "@/styles/apply-appearance";
import {
  buildTheme,
  defaultDarkTheme,
  type AppTheme,
  type ColorScheme,
} from "@/styles/build-theme";

const ThemeContext = createContext<AppTheme>(defaultDarkTheme);

function resolveColorScheme(mode: ThemeMode, systemScheme: ColorScheme | null | undefined): ColorScheme {
  if (mode === "system") {
    return systemScheme === "light" ? "light" : "dark";
  }
  return mode;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const themeMode = useAppearanceStore((s) => s.themeMode);
  const appearance = useAppearanceStore(useShallow(selectAppearanceInput));
  const systemScheme = useSystemColorScheme();

  const colorScheme = resolveColorScheme(themeMode, systemScheme as ColorScheme | null | undefined);
  const theme = useMemo(() => buildTheme(colorScheme, appearance), [appearance, colorScheme]);

  useEffect(() => {
    if (Platform.OS === "web") {
      applyAppearanceCssVars(theme);
      applyRootUiFont(theme);
    }
  }, [theme]);

  return <ThemeContext.Provider value={theme}>{children}</ThemeContext.Provider>;
}

export function useTheme(): AppTheme {
  return useContext(ThemeContext);
}

export function useResolvedColorScheme(): ColorScheme {
  const theme = useTheme();
  return theme.colorScheme;
}
