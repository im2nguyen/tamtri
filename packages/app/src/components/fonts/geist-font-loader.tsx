import {
  Geist_400Regular,
  Geist_500Medium,
  Geist_600SemiBold,
  Geist_700Bold,
  useFonts,
} from "@expo-google-fonts/geist";
import {
  GeistMono_400Regular,
  GeistMono_500Medium,
  GeistMono_600SemiBold,
} from "@expo-google-fonts/geist-mono";
import { type ReactNode } from "react";
import { ActivityIndicator, Platform, View } from "react-native";

import { useTheme } from "@/styles/use-theme";

/** Loads Geist on native; web uses Google Fonts from +html.tsx. */
export function GeistFontLoader({ children }: { children: ReactNode }) {
  const theme = useTheme();
  const [loaded] = useFonts({
    Geist_400Regular,
    Geist_500Medium,
    Geist_600SemiBold,
    Geist_700Bold,
    GeistMono_400Regular,
    GeistMono_500Medium,
    GeistMono_600SemiBold,
  });

  if (Platform.OS === "web" || loaded) {
    return children;
  }

  return (
    <View
      style={{
        flex: 1,
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: theme.colors.surface0,
      }}
    >
      <ActivityIndicator color={theme.colors.accentBright} />
    </View>
  );
}
