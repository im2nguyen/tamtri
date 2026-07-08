import { useRouter } from "expo-router";
import { AlertTriangle, CheckCircle2, ExternalLink, HelpCircle } from "lucide-react-native";
import { useCallback } from "react";
import {
  ActivityIndicator,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { Button } from "@/components/ui/button";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useHarnessHealth, type HarnessHealthEntry } from "@/hooks/use-harness-health";
import { theme } from "@/styles/theme";

function statusColor(status: string): string {
  switch (status) {
    case "ready":
      return theme.colors.accentBright;
    case "missing":
      return theme.colors.destructive;
    default:
      return theme.colors.foregroundMuted;
  }
}

function StatusIcon({ status }: { status: string }) {
  if (status === "ready") {
    return <CheckCircle2 color={statusColor(status)} size={18} />;
  }
  if (status === "missing") {
    return <AlertTriangle color={statusColor(status)} size={18} />;
  }
  return <HelpCircle color={statusColor(status)} size={18} />;
}

function HealthRow({ entry }: { entry: HarnessHealthEntry }) {
  return (
    <View
      style={{
        backgroundColor: theme.colors.surface2,
        borderRadius: theme.radius.lg,
        borderWidth: 1,
        borderColor: theme.colors.border,
        padding: theme.spacing[4],
        gap: theme.spacing[2],
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[3] }}>
        <StatusIcon status={entry.status} />
        <View style={{ flex: 1 }}>
          <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{entry.display_name}</Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }}>
            {entry.command}
          </Text>
        </View>
        <Text
          style={{
            color: statusColor(entry.status),
            fontSize: theme.fontSize.xs,
            fontWeight: "700",
            textTransform: "uppercase",
          }}
        >
          {entry.status}
        </Text>
      </View>
      {entry.status !== "ready" && entry.install_doc_url ? (
        <Pressable
          onPress={() => void Linking.openURL(entry.install_doc_url)}
          style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2], marginTop: theme.spacing[1] }}
        >
          <ExternalLink color={theme.colors.accentBright} size={14} />
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>Install guide</Text>
        </Pressable>
      ) : null}
    </View>
  );
}

export default function HealthScreen() {
  const router = useRouter();
  const { entries, checklist, loading, error, refresh } = useHarnessHealth();

  const copyChecklist = useCallback(async () => {
    if (!checklist) return;
    if (Platform.OS === "web" && typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(checklist);
    }
  }, [checklist]);

  const readyCount = entries.filter((e) => e.status === "ready").length;

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <TitlebarDragRegion />
      <ScrollView contentContainerStyle={{ padding: theme.spacing[6], alignItems: "center" }}>
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[4] }}>
          <Pressable onPress={() => router.back()}>
            <Text style={{ color: theme.colors.accentBright }}>← Back</Text>
          </Pressable>

          <View>
            <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
              Harness health
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, marginTop: theme.spacing[2], lineHeight: 22 }}>
              tamtri needs at least one installed agent. Detect what is on this machine and share the checklist with IT if
              you need help.
            </Text>
          </View>

          {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

          {loading ? (
            <ActivityIndicator color={theme.colors.accentBright} />
          ) : (
            <>
              <View
                style={{
                  padding: theme.spacing[4],
                  backgroundColor: readyCount > 0 ? theme.colors.surface2 : theme.colors.surface1,
                  borderRadius: theme.radius.lg,
                  borderWidth: 1,
                  borderColor: readyCount > 0 ? theme.colors.accent : theme.colors.border,
                }}
              >
                <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>
                  {readyCount > 0
                    ? `${readyCount} agent${readyCount === 1 ? "" : "s"} ready`
                    : "No agents detected yet"}
                </Text>
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, marginTop: 4 }}>
                  {readyCount > 0
                    ? "You can start a conversation from the sidebar."
                    : "Install a harness below, then refresh this screen."}
                </Text>
              </View>

              <View style={{ gap: theme.spacing[3] }}>
                {entries.map((entry) => (
                  <HealthRow key={entry.id} entry={entry} />
                ))}
              </View>

              <View style={{ gap: theme.spacing[3] }}>
                <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
                  <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>IT / admin checklist</Text>
                  <View style={{ flexDirection: "row", gap: theme.spacing[2] }}>
                    <Button label="Refresh" variant="ghost" compact onPress={() => void refresh()} />
                    {Platform.OS === "web" ? (
                      <Button label="Copy" variant="secondary" compact onPress={() => void copyChecklist()} />
                    ) : null}
                  </View>
                </View>
                <TextInput
                  value={checklist}
                  multiline
                  editable={false}
                  selectTextOnFocus
                  style={{
                    minHeight: 200,
                    borderWidth: 1,
                    borderColor: theme.colors.border,
                    borderRadius: theme.radius.md,
                    padding: theme.spacing[3],
                    color: theme.colors.foregroundMuted,
                    fontFamily: "monospace",
                    fontSize: theme.fontSize.xs,
                    backgroundColor: theme.colors.surface1,
                  }}
                />
              </View>
            </>
          )}
        </View>
      </ScrollView>
    </View>
  );
}
