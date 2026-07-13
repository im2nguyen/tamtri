import { useRouter } from "expo-router";
import { useCallback, useState } from "react";
import { Text, View } from "react-native";
import { method } from "@tamtri/protocol";

import { ErrorCard } from "@/components/errors/error-card";
import { SettingsCard, SettingsSection } from "@/components/settings/settings-section";
import { Button } from "@/components/ui/button";
import type { ImportResult } from "@/lib/daemon-types";
import { classifyDaemonError } from "@/lib/errors";
import { shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";
import { useTheme } from "@/styles/use-theme";

export function ImportBundleSection() {
  const theme = useTheme();
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
    <SettingsSection title="Bundle or conversation folder">
      <SettingsCard>
        <View style={{ gap: theme.spacing[4], padding: theme.spacing[3] }}>
          <Button
            label={importing ? "Importing…" : "Choose file or folder"}
            disabled={importing}
            onPress={() => void pickAndImport()}
          />

          {error ? <ErrorCard error={classifyDaemonError(error)} compact /> : null}

          {result ? (
            <View style={{ gap: theme.spacing[3] }}>
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>
                Imported “{result.conversation.title}”
              </Text>
              {result.warnings.length > 0 ? (
                <View style={{ gap: theme.spacing[2] }}>
                  {result.warnings.map((warning, index) => (
                    <Text
                      key={`${warning.kind}-${warning.detail}`}
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
      </SettingsCard>
    </SettingsSection>
  );
}
