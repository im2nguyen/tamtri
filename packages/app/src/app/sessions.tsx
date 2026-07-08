import { useRouter } from "expo-router";
import { useCallback, useEffect, useState } from "react";
import { ActivityIndicator, Pressable, ScrollView, Text, View } from "react-native";
import { method, type ConversationDto, type NativeSessionSummary } from "@tamtri/protocol";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { Button } from "@/components/ui/button";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useDaemon } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

function defaultHarness(provider: string): { harnessId: string; modelId: string } {
  return provider === "codex"
    ? { harnessId: "codex-native", modelId: "default" }
    : { harnessId: "claude-native", modelId: "default" };
}

export default function SessionsScreen() {
  const router = useRouter();
  const { client, serverInfo } = useDaemon();
  const [sessions, setSessions] = useState<NativeSessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [importingPath, setImportingPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!serverInfo.features?.session_import) {
      setLoading(false);
      return;
    }
    void (async () => {
      try {
        const rows = await client.request<NativeSessionSummary[]>(method.SESSIONS_LIST_NATIVE);
        setSessions(rows);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    })();
  }, [client, serverInfo.features?.session_import]);

  const importSession = useCallback(
    async (session: NativeSessionSummary) => {
      setImportingPath(session.path);
      try {
        const defaults = defaultHarness(session.provider);
        const imported = await client.request<ConversationDto>(method.SESSIONS_IMPORT, {
          provider: session.provider,
          path: session.path,
          harness_id: defaults.harnessId,
          model_id: defaults.modelId,
        });
        router.replace(`/conversation/${imported.id}`);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setImportingPath(null);
      }
    },
    [client, router],
  );

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <TitlebarDragRegion />
      <View style={{ padding: theme.spacing[6], alignItems: "center" }}>
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH }}>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
            Import native sessions
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, marginTop: theme.spacing[2], marginBottom: theme.spacing[4] }}>
            On-ramp terminal-started Claude and Codex sessions into your vault.
          </Text>
          <Pressable onPress={() => router.back()} style={{ marginBottom: theme.spacing[4] }}>
            <Text style={{ color: theme.colors.accentBright }}>← Back</Text>
          </Pressable>

          {error ? (
            <Text style={{ color: theme.colors.destructive, marginBottom: theme.spacing[3] }}>{error}</Text>
          ) : null}

          {loading ? (
            <ActivityIndicator color={theme.colors.accentBright} />
          ) : sessions.length === 0 ? (
            <Text style={{ color: theme.colors.foregroundMuted }}>No native sessions found under ~/.claude or ~/.codex.</Text>
          ) : (
            <ScrollView contentContainerStyle={{ gap: theme.spacing[3] }}>
              {sessions.map((session) => (
                <View
                  key={session.path}
                  style={{
                    backgroundColor: theme.colors.surface2,
                    borderRadius: theme.radius.lg,
                    borderWidth: 1,
                    borderColor: theme.colors.border,
                    padding: theme.spacing[4],
                    flexDirection: "row",
                    alignItems: "center",
                    gap: theme.spacing[4],
                  }}
                >
                  <View style={{ flex: 1 }}>
                    <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{session.title}</Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
                      {session.provider} · {session.cwd ?? "unknown cwd"}
                    </Text>
                  </View>
                  <Button
                    compact
                    label={importingPath === session.path ? "Importing…" : "Import"}
                    disabled={importingPath === session.path}
                    onPress={() => void importSession(session)}
                  />
                </View>
              ))}
            </ScrollView>
          )}
        </View>
      </View>
    </View>
  );
}
