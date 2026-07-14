import { usePathname, useRouter } from "expo-router";
import {
  ArrowLeft,
  Download,
  Gauge,
  HardDrive,
  Palette,
  Plug,
  Settings2,
  Stethoscope,
  Upload,
  type LucideIcon,
} from "lucide-react-native";
import { useMemo, useState } from "react";
import { Pressable, ScrollView, Text, useWindowDimensions, View } from "react-native";
import Animated from "react-native-reanimated";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { SidebarResizeHandle } from "@/components/layout/sidebar-resize-handle";
import { SearchInput } from "@/components/ui/search-input";
import { clampLeftSidebarWidth, isCompact } from "@/constants/layout";
import { useResizableSidebarWidth } from "@/hooks/use-resizable-sidebar-width";
import {
  sectionFromPathname,
  SETTINGS_NAV_GROUPS,
  SETTINGS_NAV_ITEMS,
  type SettingsSectionSlug,
} from "@/lib/settings-navigation";
import {
  rankSettingsSearchEntries,
  settingsSearchEntryTarget,
  settingsSectionLabel,
  type SettingsSearchEntry,
} from "@/lib/settings-search-index";
import { useSidebarRowStyles } from "@/components/sidebar/sidebar-row-styles";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

const ICONS: Record<(typeof SETTINGS_NAV_ITEMS)[number]["icon"], LucideIcon> = {
  general: Settings2,
  appearance: Palette,
  providers: Stethoscope,
  usage: Gauge,
  connect: Plug,
  import: Upload,
  sessions: Download,
  advanced: HardDrive,
};

interface SettingsSidebarNavProps {
  onClose?: () => void;
}

export function SettingsSidebarNav({ onClose }: SettingsSidebarNavProps) {
  const router = useRouter();
  const pathname = usePathname();
  const theme = useTheme();
  const rowStyles = useSidebarRowStyles();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const sidebarWidth = useUiStore((state) => state.sidebarWidth);
  const setSidebarWidth = useUiStore((state) => state.setSidebarWidth);
  const activeSection = sectionFromPathname(pathname);
  const { animatedStyle, resizeGesture, handleCursorStyle } = useResizableSidebarWidth({
    width: sidebarWidth,
    setWidth: setSidebarWidth,
    clampWidth: clampLeftSidebarWidth,
    edge: "right",
    enabled: !compact,
  });
  const [query, setQuery] = useState("");
  const results = useMemo(() => rankSettingsSearchEntries(query), [query]);
  const searching = query.trim().length > 0;

  const navigate = (section: SettingsSectionSlug, target?: string | null) => {
    const href = target
      ? `/settings/${section}?target=${encodeURIComponent(target)}`
      : `/settings/${section}`;
    router.push(href as never);
    setQuery("");
    onClose?.();
  };

  const selectResult = (entry: SettingsSearchEntry) => {
    navigate(entry.section, settingsSearchEntryTarget(entry));
  };

  const navBody = (
    <View
      style={{
        flex: 1,
        width: compact ? "100%" : undefined,
        position: "relative",
        backgroundColor: theme.colors.surfaceSidebar,
        paddingTop: compact ? 12 : 38,
      }}
    >
      <TitlebarDragRegion />
      <View style={{ paddingHorizontal: theme.spacing[2], paddingBottom: theme.spacing[3] }}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Back to app"
          onPress={() => {
            router.replace("/");
            onClose?.();
          }}
          style={({ pressed }) => [
            rowStyles.row,
            pressed ? rowStyles.pressed : null,
            { gap: theme.spacing[2] },
          ]}
        >
          <ArrowLeft color={theme.colors.foregroundMuted} size={15} />
          <Text style={rowStyles.label}>Back to app</Text>
        </Pressable>
      </View>

      <View style={{ paddingHorizontal: theme.spacing[3], paddingBottom: theme.spacing[3] }}>
        <SearchInput
          value={query}
          onChangeText={setQuery}
          onClear={() => setQuery("")}
          placeholder="Search settings…"
          returnKeyType="go"
          onSubmitEditing={() => {
            const first = results[0];
            if (first) selectResult(first);
          }}
          containerStyle={{ backgroundColor: "transparent" }}
        />
      </View>

      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{ paddingHorizontal: theme.spacing[2], paddingBottom: theme.spacing[6] }}
      >
        {searching ? (
          results.length > 0 ? (
            <View accessibilityLabel="Settings search results" style={{ gap: 2 }}>
              {results.map((entry) => {
                const item = SETTINGS_NAV_ITEMS.find((candidate) => candidate.id === entry.section);
                const Icon = item ? ICONS[item.icon] : Settings2;
                return (
                  <Pressable
                    key={entry.id}
                    accessibilityRole="button"
                    onPress={() => selectResult(entry)}
                    style={({ pressed }) => [
                      rowStyles.row,
                      pressed ? rowStyles.pressed : null,
                      { alignItems: "flex-start", paddingVertical: theme.spacing[2] },
                    ]}
                  >
                    <Icon color={theme.colors.foregroundMuted} size={15} />
                    <View style={{ flex: 1, minWidth: 0 }}>
                      <Text numberOfLines={1} style={rowStyles.label}>
                        {entry.title}
                      </Text>
                      <Text
                        numberOfLines={1}
                        style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}
                      >
                        {settingsSectionLabel(entry.section)}
                      </Text>
                    </View>
                  </Pressable>
                );
              })}
            </View>
          ) : (
            <Text style={[rowStyles.sectionLabel, { paddingHorizontal: theme.spacing[2] }]}>
              No matching settings.
            </Text>
          )
        ) : (
          <View accessibilityLabel="Settings sections" style={{ gap: theme.spacing[4] }}>
            {SETTINGS_NAV_GROUPS.map((group) => {
              const items = SETTINGS_NAV_ITEMS.filter((item) => item.group === group.id);
              return (
                <View key={group.id} style={{ gap: 2 }}>
                  <Text
                    style={[
                      rowStyles.sectionLabel,
                      { paddingHorizontal: theme.spacing[2], paddingBottom: theme.spacing[1] },
                    ]}
                  >
                    {group.label}
                  </Text>
                  {items.map((item) => {
                    const Icon = ICONS[item.icon];
                    const selected = item.id === activeSection;
                    return (
                      <Pressable
                        key={item.id}
                        accessibilityRole="button"
                        accessibilityState={{ selected }}
                        onPress={() => navigate(item.id)}
                        style={({ pressed }) => [
                          rowStyles.row,
                          selected ? rowStyles.active : null,
                          pressed ? rowStyles.pressed : null,
                        ]}
                      >
                        <Icon
                          color={selected ? theme.colors.foreground : theme.colors.foregroundMuted}
                          size={15}
                        />
                        <Text
                          style={[
                            rowStyles.label,
                            {
                              color: selected
                                ? theme.colors.foreground
                                : theme.colors.foregroundMuted,
                              fontWeight: selected ? "500" : "400",
                            },
                          ]}
                        >
                          {item.label}
                        </Text>
                      </Pressable>
                    );
                  })}
                </View>
              );
            })}
          </View>
        )}
      </ScrollView>

      {!compact ? (
        <SidebarResizeHandle edge="right" gesture={resizeGesture} cursorStyle={handleCursorStyle} />
      ) : null}
    </View>
  );

  if (compact) {
    return navBody;
  }

  return (
    <Animated.View style={[{ flexShrink: 0, alignSelf: "stretch" }, animatedStyle]}>
      {navBody}
    </Animated.View>
  );
}
