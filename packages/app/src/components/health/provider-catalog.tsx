import { ExternalLink, PackagePlus, Search } from "lucide-react-native";
import { useMemo, useState } from "react";
import { Linking, Platform, Pressable, Text, TextInput, View } from "react-native";

import { Button } from "@/components/ui/button";
import { ACP_PROVIDER_CATALOG, type AcpProviderCatalogEntry } from "@/data/acp-provider-catalog";
import { useTheme } from "@/styles/use-theme";

function matchesSearch(entry: AcpProviderCatalogEntry, query: string): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return [entry.title, entry.id, entry.description].some((value) =>
    value.toLowerCase().includes(normalized),
  );
}

interface CatalogRowProps {
  entry: AcpProviderCatalogEntry;
  installing: boolean;
  onInstall: (entry: AcpProviderCatalogEntry) => void;
}

function CatalogRow({ entry, installing, onInstall }: CatalogRowProps) {
  const theme = useTheme();
  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[3],
        padding: theme.spacing[3],
        borderBottomWidth: 1,
        borderBottomColor: theme.colors.border,
      }}
    >
      <View
        style={{
          width: 36,
          height: 36,
          borderRadius: theme.radius.md,
          backgroundColor: theme.colors.surface2,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <PackagePlus color={theme.colors.foreground} size={18} />
      </View>
      <View style={{ flex: 1, minWidth: 0, gap: theme.spacing[1] }}>
        <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
          <Text style={{ color: theme.colors.foreground, fontWeight: "600", fontSize: theme.fontSize.sm }} numberOfLines={1}>
            {entry.title}
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>{entry.version}</Text>
        </View>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }} numberOfLines={1}>
          {entry.description}
        </Text>
        <Pressable
          onPress={() => void Linking.openURL(entry.installLink)}
          style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}
        >
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>Install instructions</Text>
          <ExternalLink color={theme.colors.foregroundMuted} size={12} />
        </Pressable>
      </View>
      <Button
        label={installing ? "Adding…" : "Add"}
        compact
        onPress={() => onInstall(entry)}
        style={{ minWidth: 72, opacity: installing ? 0.6 : 1 }}
      />
    </View>
  );
}

interface ProviderCatalogProps {
  installedIds: Set<string>;
  installingId: string | null;
  onInstall: (entry: AcpProviderCatalogEntry) => Promise<void>;
}

export function ProviderCatalog({ installedIds, installingId, onInstall }: ProviderCatalogProps) {
  const theme = useTheme();
  const [search, setSearch] = useState("");

  const availableEntries = useMemo(
    () =>
      ACP_PROVIDER_CATALOG.filter((entry) => !installedIds.has(entry.id)).filter((entry) =>
        matchesSearch(entry, search),
      ),
    [installedIds, search],
  );

  return (
    <View>
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[2],
          backgroundColor: theme.colors.surface2,
          borderRadius: theme.radius.lg,
          borderWidth: 1,
          borderColor: theme.colors.border,
          paddingHorizontal: theme.spacing[3],
          marginBottom: theme.spacing[3],
        }}
      >
        <Search color={theme.colors.foregroundMuted} size={16} />
        <TextInput
          value={search}
          onChangeText={setSearch}
          placeholder="Search catalog…"
          placeholderTextColor={theme.colors.foregroundMuted}
          autoCapitalize="none"
          autoCorrect={false}
          style={[
            {
              flex: 1,
              paddingVertical: theme.spacing[3],
              color: theme.colors.foreground,
              fontSize: theme.fontSize.sm,
            },
            Platform.OS === "web"
              ? ({ outlineStyle: "none", outlineWidth: 0 } as Record<string, unknown>)
              : null,
          ]}
        />
      </View>

      {availableEntries.length === 0 ? (
        <View
          style={{
            minHeight: 96,
            borderRadius: theme.radius.lg,
            borderWidth: 1,
            borderColor: theme.colors.border,
            backgroundColor: theme.colors.surface1,
            alignItems: "center",
            justifyContent: "center",
            padding: theme.spacing[4],
          }}
        >
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
            No catalog providers match your search.
          </Text>
        </View>
      ) : (
        <View
          style={{
            borderRadius: theme.radius.lg,
            borderWidth: 1,
            borderColor: theme.colors.border,
            overflow: "hidden",
            backgroundColor: theme.colors.surface1,
          }}
        >
          {availableEntries.map((entry) => (
            <CatalogRow
              key={entry.id}
              entry={entry}
              installing={installingId === entry.id}
              onInstall={(item) => void onInstall(item)}
            />
          ))}
        </View>
      )}
    </View>
  );
}
