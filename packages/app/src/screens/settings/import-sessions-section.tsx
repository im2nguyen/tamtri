import { useRouter } from "expo-router";
import { useCallback, useEffect, useState } from "react";
import { ActivityIndicator, ScrollView, Text, View } from "react-native";
import { method, type ConversationDto, type NativeSessionSummary } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { useDaemon } from "@/runtime/daemon-provider";
import { useTheme } from "@/styles/use-theme";

function defaultHarness(provider: string): { harnessId: string; modelId: string } {
  return provider === "codex"
    ? { harnessId: "codex-native", modelId: "default" }
    : { harnessId: "claude-native", modelId: "default" };
}

export function ImportSessionsSection() {
  const theme = useTheme();
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

  if (!serverInfo.features?.session_import) {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, lineHeight: 22 }}>
        Native session import requires a newer tamtri daemon.
      </Text>
    );
  }

  if (loading) {
    return <ActivityIndicator color={theme.colors.accentBright} />;
  }

  if (sessions.length === 0) {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, lineHeight: 22 }}>
        No native sessions found under ~/.claude or ~/.codex.
      </Text>
    );
  }

  return (
    <View style={{ gap: theme.spacing[3] }}>
      {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}
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
    </View>
  );
}
