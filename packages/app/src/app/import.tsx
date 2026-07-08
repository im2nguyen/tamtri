import { useRouter } from "expo-router";
import { useCallback, useState } from "react";
import { Pressable, ScrollView, Text, View } from "react-native";
import { method } from "@tamtri/protocol";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { ErrorCard } from "@/components/errors/error-card";
import { Button } from "@/components/ui/button";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import type { ImportResult } from "@/lib/daemon-types";
import { classifyDaemonError } from "@/lib/errors";
import { shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

export default function ImportScreen() {
  const router = useRouter();
  const { client } = useDaemon();
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pickAndImport = useCallback(async () => {
    setError(null);
    setResult(null);
    const shell = shellBridge();
    if (!shell?.pickOpenFile) {
      setError("Import requires the tamtri desktop app with file access.");
      return;
    }
    const path = await shell.pickOpenFile({
      title: "Import .tamtri bundle or conversation folder",
      filters: [
        { name: "tamtri bundle", extensions: ["tamtri"] },
        { name: "All files", extensions: ["*"] },
      ],
    });
    if (!path) return;

    setImporting(true);
    try {
      const imported = await client.request<ImportResult>(method.CONVERSATION_IMPORT, { source_path: path });
      setResult(imported);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setImporting(false);
    }
  }, [client]);

  const openImported = useCallback(() => {
    if (!result) return;
    router.replace(`/conversation/${result.conversation.id}`);
  }, [result, router]);

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
              Import conversation
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, marginTop: theme.spacing[2], lineHeight: 22 }}>
              Open a `.tamtri` bundle or conversation folder from your vault. Attachments are hash-verified on import.
            </Text>
          </View>

          <Button
            label={importing ? "Importing…" : "Choose file or folder"}
            disabled={importing}
            onPress={() => void pickAndImport()}
          />

          {error ? <ErrorCard error={classifyDaemonError(error)} compact /> : null}

          {result ? (
            <View
              style={{
                backgroundColor: theme.colors.surface2,
                borderRadius: theme.radius.lg,
                padding: theme.spacing[4],
                gap: theme.spacing[3],
                borderWidth: 1,
                borderColor: theme.colors.border,
              }}
            >
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>
                Imported “{result.conversation.title}”
              </Text>
              {result.warnings.length > 0 ? (
                <View style={{ gap: theme.spacing[2] }}>
                  {result.warnings.map((warning, index) => (
                    <Text
                      key={`${warning.kind}-${index}`}
                      style={{ color: theme.colors.destructive, fontSize: theme.fontSize.sm }}
                    >
                      {warning.kind}: {warning.detail}
                    </Text>
                  ))}
                </View>
              ) : (
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
                  All attachments passed integrity checks.
                </Text>
              )}
              <Button label="Open conversation" onPress={openImported} />
            </View>
          ) : null}
        </View>
      </ScrollView>
    </View>
  );
}
