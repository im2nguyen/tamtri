import { ExternalLink, Search } from "lucide-react-native";
import { useEffect, useMemo, useState } from "react";
import { Linking, Modal, Platform, Pressable, ScrollView, Text, TextInput, View } from "react-native";

import { Button } from "@/components/ui/button";
import { AGENT_CATALOG, type AgentCatalogEntry } from "@/data/agent-catalog";
import { useTheme } from "@/styles/use-theme";

interface AddAgentSheetProps {
  visible: boolean;
  installedIds: Set<string>;
  installingId: string | null;
  onClose: () => void;
  onAdd: (entry: AgentCatalogEntry) => Promise<void>;
}

function matchesSearch(entry: AgentCatalogEntry, query: string): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return [entry.title, entry.id, entry.description].some((value) =>
    value.toLowerCase().includes(normalized),
  );
}

function adapterLabel(adapter: AgentCatalogEntry["adapter"]): string {
  return adapter === "acp" ? "ACP" : "Native";
}

function CatalogPickRow({
  entry,
  selected,
  installed,
  onPress,
}: {
  entry: AgentCatalogEntry;
  selected: boolean;
  installed: boolean;
  onPress: () => void;
}) {
  const theme = useTheme();
  return (
    <Pressable
      onPress={onPress}
      style={({ pressed }) => ({
        paddingHorizontal: theme.spacing[3],
        paddingVertical: theme.spacing[3],
        borderRadius: theme.radius.lg,
        backgroundColor: selected
          ? theme.colors.surface3
          : pressed
            ? theme.colors.surface2
            : "transparent",
        borderWidth: 1,
        borderColor: selected ? theme.colors.accent : "transparent",
        opacity: installed ? 0.55 : 1,
      })}
    >
      <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: theme.spacing[2] }}>
        <View style={{ flex: 1, minWidth: 0 }}>
          <Text style={{ color: theme.colors.foreground, fontWeight: "600", fontSize: theme.fontSize.sm }} numberOfLines={1}>
            {entry.title}
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }} numberOfLines={1}>
            {adapterLabel(entry.adapter)}
          </Text>
        </View>
        {installed ? (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>Added</Text>
        ) : null}
      </View>
    </Pressable>
  );
}

export function AddAgentSheet({
  visible,
  installedIds,
  installingId,
  onClose,
  onAdd,
}: AddAgentSheetProps) {
  const theme = useTheme();
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<AgentCatalogEntry | null>(null);

  const availableEntries = useMemo(() => {
    const featured = AGENT_CATALOG.filter((entry) => entry.featured);
    const rest = AGENT_CATALOG.filter((entry) => !entry.featured);
    return [...featured, ...rest].filter((entry) => matchesSearch(entry, search));
  }, [search]);

  useEffect(() => {
    if (!visible) {
      setSearch("");
      setSelected(null);
      return;
    }
    const first = availableEntries.find((entry) => !installedIds.has(entry.id)) ?? availableEntries[0] ?? null;
    setSelected(first);
  }, [availableEntries, installedIds, visible]);

  const selectedInstalled = selected ? installedIds.has(selected.id) : false;
  const adding = selected ? installingId === selected.id : false;

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <Pressable
        onPress={onClose}
        style={{
          flex: 1,
          backgroundColor: "rgba(0,0,0,0.55)",
          justifyContent: "center",
          padding: theme.spacing[6],
        }}
      >
        <Pressable
          onPress={(event) => event.stopPropagation()}
          style={{
            alignSelf: "center",
            width: "100%",
            maxWidth: 720,
            maxHeight: "85%",
            backgroundColor: theme.colors.surface1,
            borderRadius: theme.radius.xl,
            borderWidth: 1,
            borderColor: theme.colors.border,
            overflow: "hidden",
          }}
        >
          <View style={{ padding: theme.spacing[5], gap: theme.spacing[2], borderBottomWidth: 1, borderBottomColor: theme.colors.border }}>
            <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
              Add agent app
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
              Pick an agent, follow the install steps, then add it to tamtri. Claude, Codex, OpenCode, and Pi use native adapters when available.
            </Text>
          </View>

          <View style={{ flexDirection: "row", flex: 1, minHeight: 360 }}>
            <View
              style={{
                width: 280,
                borderRightWidth: 1,
                borderRightColor: theme.colors.border,
                padding: theme.spacing[4],
                gap: theme.spacing[3],
              }}
            >
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
                }}
              >
                <Search color={theme.colors.foregroundMuted} size={16} />
                <TextInput
                  value={search}
                  onChangeText={setSearch}
                  placeholder="Search agents…"
                  placeholderTextColor={theme.colors.foregroundMuted}
                  autoCapitalize="none"
                  autoCorrect={false}
                  style={[
                    {
                      flex: 1,
                      paddingVertical: theme.spacing[2],
                      color: theme.colors.foreground,
                      fontSize: theme.fontSize.sm,
                    },
                    Platform.OS === "web"
                      ? ({ outlineStyle: "none", outlineWidth: 0 } as Record<string, unknown>)
                      : null,
                  ]}
                />
              </View>

              <ScrollView contentContainerStyle={{ gap: theme.spacing[1] }}>
                {availableEntries.length === 0 ? (
                  <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
                    No agents match your search.
                  </Text>
                ) : (
                  availableEntries.map((entry) => (
                    <CatalogPickRow
                      key={entry.id}
                      entry={entry}
                      selected={selected?.id === entry.id}
                      installed={installedIds.has(entry.id)}
                      onPress={() => setSelected(entry)}
                    />
                  ))
                )}
              </ScrollView>
            </View>

            <View style={{ flex: 1, padding: theme.spacing[5], gap: theme.spacing[4] }}>
              {selected ? (
                <>
                  <View style={{ gap: theme.spacing[1] }}>
                    <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "700" }}>
                      {selected.title}
                    </Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                      {adapterLabel(selected.adapter)} · {selected.command}
                      {selected.args.length > 0 ? ` ${selected.args.join(" ")}` : ""}
                    </Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 22, marginTop: theme.spacing[2] }}>
                      {selected.description}
                    </Text>
                  </View>

                  <View
                    style={{
                      padding: theme.spacing[4],
                      borderRadius: theme.radius.lg,
                      backgroundColor: theme.colors.surface2,
                      borderWidth: 1,
                      borderColor: theme.colors.border,
                      gap: theme.spacing[2],
                    }}
                  >
                    <Text style={{ color: theme.colors.foreground, fontWeight: "600", fontSize: theme.fontSize.sm }}>
                      Install instructions
                    </Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 22 }}>
                      {selected.installSteps}
                    </Text>
                    <Pressable
                      onPress={() => void Linking.openURL(selected.installLink)}
                      style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1], marginTop: theme.spacing[1] }}
                    >
                      <ExternalLink color={theme.colors.accentBright} size={14} />
                      <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm, fontWeight: "600" }}>
                        Open install guide
                      </Text>
                    </Pressable>
                  </View>

                  <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[3], marginTop: "auto" }}>
                    <Button label="Cancel" variant="ghost" onPress={onClose} />
                    <Button
                      label={selectedInstalled ? "Already added" : adding ? "Adding…" : "Add to tamtri"}
                      disabled={selectedInstalled || adding}
                      onPress={() => void onAdd(selected)}
                    />
                  </View>
                </>
              ) : (
                <Text style={{ color: theme.colors.foregroundMuted }}>Select an agent to see install instructions.</Text>
              )}
            </View>
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}
