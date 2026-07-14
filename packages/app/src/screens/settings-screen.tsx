import { useRouter } from "expo-router";
import { ArrowLeft, PanelLeft } from "lucide-react-native";
import { useEffect, useMemo } from "react";
import { Platform, Pressable, ScrollView, Text, useWindowDimensions, View } from "react-native";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { SurfaceChip } from "@/components/ui/surface-controls";
import { MAX_CONTENT_WIDTH, isCompact } from "@/constants/layout";
import {
  SETTINGS_NAV_ITEMS,
  type SettingsSectionSlug,
} from "@/lib/settings-navigation";
import { AdvancedSection } from "@/screens/settings/advanced-section";
import { ProvidersSection } from "@/screens/settings/agents-providers-section";
import { AppearanceSection } from "@/screens/settings/appearance-section";
import { ConnectHostSection } from "@/screens/settings/connect-host-section";
import { GeneralSection } from "@/screens/settings/general-section";
import { ImportBundleSection } from "@/screens/settings/import-bundle-section";
import { ImportSessionsSection } from "@/screens/settings/import-sessions-section";
import { UsageSection } from "@/screens/settings/usage-section";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface SettingsScreenProps {
  section: SettingsSectionSlug;
  target?: string;
}

function SectionContent({ section }: { section: SettingsSectionSlug }) {
  switch (section) {
    case "general":
      return <GeneralSection />;
    case "appearance":
      return <AppearanceSection />;
    case "providers":
      return <ProvidersSection />;
    case "usage":
      return <UsageSection />;
    case "connect":
      return <ConnectHostSection />;
    case "import":
      return <ImportBundleSection />;
    case "sessions":
      return <ImportSessionsSection />;
    case "advanced":
      return <AdvancedSection />;
  }
}

export function SettingsScreen({ section, target }: SettingsScreenProps) {
  const router = useRouter();
  const theme = useTheme();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const setSidebarOpen = useUiStore((state) => state.setSidebarOpen);
  const meta = useMemo(
    () => SETTINGS_NAV_ITEMS.find((item) => item.id === section) ?? SETTINGS_NAV_ITEMS[0]!,
    [section],
  );

  useEffect(() => {
    if (!target || Platform.OS !== "web" || typeof document === "undefined") return;
    const frame = requestAnimationFrame(() => {
      document.getElementById(target)?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
    return () => cancelAnimationFrame(frame);
  }, [section, target]);

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surfaceWorkspace }}>
      <TitlebarDragRegion />
      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{
          paddingHorizontal: compact ? theme.spacing[4] : theme.spacing[6],
          paddingTop: compact ? theme.spacing[3] : theme.spacing[8],
          paddingBottom: theme.spacing[8],
          alignItems: "center",
        }}
      >
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[5] }}>
          {compact ? (
            <View style={{ gap: theme.spacing[3] }}>
              <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel="Open settings navigation"
                  onPress={() => setSidebarOpen(true)}
                  hitSlop={8}
                >
                  <PanelLeft color={theme.colors.foregroundMuted} size={18} />
                </Pressable>
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel="Back to app"
                  onPress={() => router.replace("/")}
                  style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}
                >
                  <ArrowLeft color={theme.colors.foregroundMuted} size={15} />
                  <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                    Back to app
                  </Text>
                </Pressable>
              </View>
              <ScrollView
                horizontal
                showsHorizontalScrollIndicator={false}
                contentContainerStyle={{ gap: theme.spacing[1] }}
              >
                {SETTINGS_NAV_ITEMS.map((item) => (
                  <SurfaceChip
                    key={item.id}
                    label={item.label}
                    selected={item.id === section}
                    onPress={() => router.push(`/settings/${item.id}` as never)}
                  />
                ))}
              </ScrollView>
            </View>
          ) : null}

          <View style={{ gap: theme.spacing[1] }}>
            <Text
              accessibilityRole="header"
              style={{
                color: theme.colors.foreground,
                fontSize: theme.fontSize.lg,
                fontWeight: "600",
              }}
            >
              {meta.label}
            </Text>
            <Text
              style={{
                color: theme.colors.foregroundMuted,
                fontSize: theme.fontSize.sm,
                lineHeight: 20,
              }}
            >
              {meta.description}
            </Text>
          </View>

          <SectionContent section={section} />
        </View>
      </ScrollView>
    </View>
  );
}
