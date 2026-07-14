import { ChevronDown, ChevronUp, ExternalLink, RefreshCw } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ActivityIndicator,
  Linking,
  Pressable,
  Text,
  View,
} from "react-native";

import { AddAgentSheet } from "@/components/health/add-agent-sheet";
import { ProviderRow } from "@/components/health/provider-row";
import { Disclosure } from "@/components/ui/disclosure";
import { SettingsCard, SettingsSection } from "@/components/settings/settings-section";
import { Button } from "@/components/ui/button";
import { SearchInput } from "@/components/ui/search-input";
import { Switch } from "@/components/ui/switch";
import type { AgentCatalogEntry } from "@/data/agent-catalog";
import { useHarnessPickerSettings } from "@/hooks/use-harness-picker-settings";
import { useHarnessProviders } from "@/hooks/use-harness-providers";
import { useTheme } from "@/styles/use-theme";
import { method } from "@tamtri/protocol";
import { useDaemon } from "@/runtime/daemon-provider";

interface InstalledCli {
  id: string;
  display_name: string;
  command: string;
  version?: string | null;
  install_doc_url: string;
  in_roster: boolean;
  auth_ready: boolean;
}

function matchesProviderSearch(
  entry: { display_name: string; id: string; command: string },
  query: string,
): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return [entry.display_name, entry.id, entry.command].some((value) =>
    value.toLowerCase().includes(normalized),
  );
}

function moveItem(order: string[], id: string, direction: -1 | 1): string[] {
  const index = order.indexOf(id);
  if (index < 0) return order;
  const target = index + direction;
  if (target < 0 || target >= order.length) return order;
  const next = [...order];
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

export function ProvidersSection() {
  const theme = useTheme();
  const { client } = useDaemon();
  const { entries, loading, error, pendingId, refresh, setEnabled, addProvider } = useHarnessProviders();
  const { settings, loading: settingsLoading, pending: settingsPending, save } = useHarnessPickerSettings();
  const [search, setSearch] = useState("");
  const [addSheetOpen, setAddSheetOpen] = useState(false);
  const [installingId, setInstallingId] = useState<string | null>(null);
  const [installedClis, setInstalledClis] = useState<InstalledCli[]>([]);
  const [clisLoading, setClisLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const refreshInstalledClis = useCallback(async () => {
    setClisLoading(true);
    try {
      const rows = await client.request<InstalledCli[]>(method.HARNESS_INSTALLED_CLIS_LIST);
      setInstalledClis(rows);
    } finally {
      setClisLoading(false);
    }
  }, [client]);

  const handleRefreshAll = useCallback(async () => {
    await Promise.all([refresh(), refreshInstalledClis()]);
  }, [refresh, refreshInstalledClis]);

  const handleDiscoverySync = useCallback(async () => {
    setSyncing(true);
    try {
      await client.request(method.HARNESS_DISCOVERY_SYNC);
      await handleRefreshAll();
    } finally {
      setSyncing(false);
    }
  }, [client, handleRefreshAll]);

  useEffect(() => {
    void refreshInstalledClis();
  }, [refreshInstalledClis]);

  const order = useMemo(() => {
    if (settings?.harness_order?.length) return settings.harness_order;
    return entries.map((entry) => entry.id);
  }, [entries, settings?.harness_order]);

  const hiddenSet = useMemo(
    () => new Set(settings?.hidden_harness_ids ?? []),
    [settings?.hidden_harness_ids],
  );

  const orderedEntries = useMemo(() => {
    const byId = new Map(entries.map((entry) => [entry.id, entry]));
    const ordered = order.map((id) => byId.get(id)).filter((entry): entry is NonNullable<typeof entry> => !!entry);
    for (const entry of entries) {
      if (!order.includes(entry.id)) ordered.push(entry);
    }
    return ordered;
  }, [entries, order]);

  const filteredEntries = useMemo(
    () => orderedEntries.filter((entry) => matchesProviderSearch(entry, search)),
    [orderedEntries, search],
  );

  const installedIds = useMemo(() => new Set(entries.map((entry) => entry.id)), [entries]);
  const readyCount = entries.filter((entry) => entry.enabled && entry.status === "ready").length;

  const handleAddAgent = useCallback(
    async (entry: AgentCatalogEntry) => {
      setInstallingId(entry.id);
      try {
        await addProvider({
          id: entry.id,
          display_name: entry.title,
          command: entry.command,
          args: [...entry.args],
          env: entry.env,
          adapter: entry.adapter,
        });
        setAddSheetOpen(false);
        await handleRefreshAll();
      } finally {
        setInstallingId(null);
      }
    },
    [addProvider, handleRefreshAll],
  );

  const updatePicker = useCallback(
    async (patch: Partial<{ harness_order: string[]; hidden_harness_ids: string[]; enable_cli_update_checks: boolean }>) => {
      if (!settings) return;
      await save({
        harness_order: patch.harness_order ?? settings.harness_order,
        hidden_harness_ids: patch.hidden_harness_ids ?? settings.hidden_harness_ids,
        enable_cli_update_checks:
          patch.enable_cli_update_checks ?? settings.enable_cli_update_checks,
      });
      await refresh();
    },
    [refresh, save, settings],
  );

  const busy = loading || settingsLoading || clisLoading || syncing || settingsPending;

  return (
    <View style={{ gap: theme.spacing[5] }}>
      <SettingsSection title="Updates">
        <SettingsCard>
          <View
            style={{
              flexDirection: "row",
              alignItems: "center",
              justifyContent: "space-between",
              padding: theme.spacing[3],
              gap: theme.spacing[3],
            }}
          >
            <View style={{ flex: 1 }}>
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>Automatic CLI update checks</Text>
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, marginTop: 4 }}>
                Check installed provider CLIs for newer versions in the background.
              </Text>
            </View>
            <Switch
              value={settings?.enable_cli_update_checks ?? true}
              disabled={!settings || settingsPending}
              accessibilityLabel="Automatic CLI update checks"
              onValueChange={(value) => void updatePicker({ enable_cli_update_checks: value })}
            />
          </View>
        </SettingsCard>
      </SettingsSection>

      <SettingsSection title="Visible providers">
        <SettingsCard>
          <View style={{ padding: theme.spacing[3], gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
              Reorder and hide providers in the composer picker. The active thread always keeps its provider visible.
            </Text>
            {orderedEntries.map((entry, index) => {
              const hidden = hiddenSet.has(entry.id);
              return (
                <View
                  key={entry.id}
                  style={{
                    flexDirection: "row",
                    alignItems: "center",
                    gap: theme.spacing[2],
                    paddingVertical: theme.spacing[2],
                    borderTopWidth: index === 0 ? 0 : 1,
                    borderTopColor: theme.colors.border,
                  }}
                >
                  <View style={{ gap: 2 }}>
                    <Pressable
                      accessibilityLabel={`Move ${entry.display_name} up`}
                      disabled={index === 0 || settingsPending}
                      onPress={() => void updatePicker({ harness_order: moveItem(order, entry.id, -1) })}
                      hitSlop={6}
                    >
                      <ChevronUp color={theme.colors.foregroundMuted} size={14} />
                    </Pressable>
                    <Pressable
                      accessibilityLabel={`Move ${entry.display_name} down`}
                      disabled={index === orderedEntries.length - 1 || settingsPending}
                      onPress={() => void updatePicker({ harness_order: moveItem(order, entry.id, 1) })}
                      hitSlop={6}
                    >
                      <ChevronDown color={theme.colors.foregroundMuted} size={14} />
                    </Pressable>
                  </View>
                  <View style={{ flex: 1 }}>
                    <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{entry.display_name}</Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>{entry.id}</Text>
                  </View>
                  <Switch
                    value={!hidden}
                    disabled={settingsPending}
                    accessibilityLabel={`Show ${entry.display_name} in picker`}
                    onValueChange={(visible) => {
                      const nextHidden = new Set(hiddenSet);
                      if (visible) nextHidden.delete(entry.id);
                      else nextHidden.add(entry.id);
                      void updatePicker({ hidden_harness_ids: [...nextHidden] });
                    }}
                  />
                </View>
              );
            })}
          </View>
        </SettingsCard>
      </SettingsSection>

      <SettingsSection title="Readiness">
        <SettingsCard>
          <View style={{ padding: theme.spacing[3] }}>
            <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>
              {readyCount > 0
                ? `${readyCount} agent app${readyCount === 1 ? "" : "s"} ready`
                : "No agent apps available yet"}
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, marginTop: 4 }}>
              {readyCount > 0
                ? "Ready agents appear in the composer and new-conversation picker."
                : "Install a provider CLI, then tap Sync discovered agents."}
            </Text>
          </View>
        </SettingsCard>
      </SettingsSection>

      <SettingsSection title="Agent apps">
        <View style={{ gap: theme.spacing[3] }}>
          <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[2] }}>
            <Pressable
              onPress={() => void handleRefreshAll()}
              accessibilityLabel="Refresh agent apps"
              accessibilityRole="button"
              hitSlop={8}
              style={({ pressed }) => ({
                width: 32,
                height: 32,
                borderRadius: theme.radius.md,
                alignItems: "center",
                justifyContent: "center",
                backgroundColor: pressed ? theme.colors.surface2 : "transparent",
              })}
            >
              <RefreshCw color={theme.colors.foregroundMuted} size={16} />
            </Pressable>
            <Button label={syncing ? "Syncing…" : "Sync discovered"} compact onPress={() => void handleDiscoverySync()} />
            <Button label="Add agent" compact onPress={() => setAddSheetOpen(true)} />
          </View>

          <SearchInput
            value={search}
            onChangeText={setSearch}
            onClear={() => setSearch("")}
            placeholder="Search agent apps…"
          />

          {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

          {busy ? (
            <ActivityIndicator color={theme.colors.accentBright} />
          ) : entries.length === 0 ? (
            <View
              style={{
                padding: theme.spacing[5],
                borderRadius: theme.radius.lg,
                borderWidth: 1,
                borderColor: theme.colors.border,
                gap: theme.spacing[3],
                alignItems: "center",
              }}
            >
              <Text style={{ color: theme.colors.foregroundMuted, textAlign: "center", lineHeight: 22 }}>
                No agent apps configured yet. Tap Sync discovered or Add agent.
              </Text>
              <Button label="Sync discovered" onPress={() => void handleDiscoverySync()} />
            </View>
          ) : (
            <View
              style={{
                borderRadius: theme.radius.lg,
                borderWidth: 1,
                borderColor: theme.colors.border,
                overflow: "hidden",
              }}
            >
              {filteredEntries.length === 0 ? (
                <View style={{ padding: theme.spacing[4] }}>
                  <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
                    No agent apps match your search.
                  </Text>
                </View>
              ) : (
                filteredEntries.map((entry, index) => (
                  <ProviderRow
                    key={entry.id}
                    entry={entry}
                    isFirst={index === 0}
                    isToggling={pendingId === entry.id}
                    onToggleEnabled={(id, enabled) => void setEnabled(id, enabled)}
                  />
                ))
              )}
            </View>
          )}
        </View>
      </SettingsSection>

      <SettingsSection title="Installed CLIs">
        <SettingsCard>
          {installedClis.length === 0 && !clisLoading ? (
            <View style={{ padding: theme.spacing[4] }}>
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
                No supported provider CLIs detected on PATH.
              </Text>
            </View>
          ) : (
            installedClis.map((cli, index) => (
              <Disclosure
                key={cli.id}
                accessibilityLabel={`${cli.display_name} CLI details`}
                title={
                  <View
                    style={{
                      flex: 1,
                      flexDirection: "row",
                      alignItems: "center",
                      justifyContent: "space-between",
                      paddingVertical: theme.spacing[2],
                      paddingHorizontal: theme.spacing[3],
                      borderTopWidth: index === 0 ? 0 : 1,
                      borderTopColor: theme.colors.border,
                    }}
                  >
                    <View style={{ flex: 1 }}>
                      <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{cli.display_name}</Text>
                      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                        {cli.version ?? "Version unknown"}
                        {cli.auth_ready ? " · Signed in" : cli.in_roster ? " · Needs sign-in" : " · Not in roster"}
                      </Text>
                    </View>
                  </View>
                }
              >
                <View style={{ paddingHorizontal: theme.spacing[3], paddingBottom: theme.spacing[3], gap: theme.spacing[2] }}>
                  <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                    Binary: {cli.command}
                  </Text>
                  <Pressable
                    onPress={() => void Linking.openURL(cli.install_doc_url)}
                    style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}
                  >
                    <ExternalLink color={theme.colors.accentBright} size={12} />
                    <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs }}>Install docs</Text>
                  </Pressable>
                </View>
              </Disclosure>
            ))
          )}
        </SettingsCard>
      </SettingsSection>

      <AddAgentSheet
        visible={addSheetOpen}
        installedIds={installedIds}
        installingId={installingId ?? pendingId}
        onClose={() => setAddSheetOpen(false)}
        onAdd={handleAddAgent}
      />
    </View>
  );
}
