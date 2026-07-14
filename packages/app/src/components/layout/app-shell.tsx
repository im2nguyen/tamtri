import { Slot, usePathname } from "expo-router";
import { useWindowDimensions, View } from "react-native";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { LeftSidebar } from "@/components/sidebar/left-sidebar";
import { SettingsSidebarNav } from "@/components/settings/settings-sidebar-nav";
import { isCompact } from "@/constants/layout";
import { useUiStore } from "@/stores/ui-store";
import { raisedContentCardStyle } from "@/styles/surface-styles";
import { useTheme } from "@/styles/use-theme";

export function AppShell() {
  const theme = useTheme();
  const pathname = usePathname();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const setSidebarOpen = useUiStore((s) => s.setSidebarOpen);
  const settingsRoute = pathname.startsWith("/settings");
  const Sidebar = settingsRoute ? SettingsSidebarNav : LeftSidebar;

  return (
    <GestureHandlerRootView style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <SafeAreaProvider>
        <View
          style={{
            flex: 1,
            flexDirection: "row",
            backgroundColor: theme.colors.surface0,
          }}
        >
          {!compact || sidebarOpen ? (
            compact ? (
              <View style={{ position: "absolute", top: 0, left: 0, right: 0, bottom: 0, zIndex: 20 }}>
                <Sidebar onClose={() => setSidebarOpen(false)} />
              </View>
            ) : (
              <Sidebar />
            )
          ) : null}
          <View
            style={[
              { flex: 1, minWidth: 0, overflow: "hidden" },
              compact ? null : raisedContentCardStyle(theme),
            ]}
          >
            <Slot />
          </View>
        </View>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}
