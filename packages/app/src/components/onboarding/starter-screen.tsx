import { useRouter } from "expo-router";
import { FileSpreadsheet, Upload } from "lucide-react-native";
import { useCallback, useState } from "react";
import {
  ActivityIndicator,
  Platform,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import { method, type ConversationDto } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import {
  onboardingCopy,
  SAMPLE_CSV,
  SAMPLE_CSV_FILENAME,
  SAMPLE_PROMPT,
} from "@/content/onboarding-copy";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useAgents } from "@/hooks/use-agents";
import { useReadiness } from "@/hooks/use-readiness";
import { encodeBase64, utf8Bytes } from "@/lib/base64";
import { useDaemon } from "@/runtime/daemon-provider";
import { useOnboardingStore } from "@/stores/onboarding-store";
import { useTheme } from "@/styles/use-theme";

export function StarterScreen() {
  const theme = useTheme();
  const router = useRouter();
  const { client } = useDaemon();
  const { loadModels } = useAgents();
  const { recommendation, readyCount, loading: readinessLoading } = useReadiness();
  const setSampleRunConversationId = useOnboardingStore((s) => s.setSampleRunConversationId);
  const setPhase = useOnboardingStore((s) => s.setPhase);
  const [prompt, setPrompt] = useState(SAMPLE_PROMPT);
  const [dragActive, setDragActive] = useState(false);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const copy = onboardingCopy.starter;

  const runSample = useCallback(async () => {
    const agentId = recommendation?.agent_id;
    if (!agentId) {
      setError("No agent app is ready. Go back and install one first.");
      return;
    }
    const trimmedPrompt = prompt.trim();
    if (!trimmedPrompt) return;

    setRunning(true);
    setError(null);
    try {
      let modelId = "default";
      try {
        const models = await loadModels(agentId);
        modelId = models[0]?.id ?? "default";
      } catch {
        /* default model */
      }

      const created = await client.request<ConversationDto>(method.CONVERSATION_CREATE, {
        title: "Sample sales report",
        harness_id: agentId,
        model_id: modelId,
      });

      await client.request(method.WORKDIR_WRITE_FILE, {
        conversation_id: created.id,
        filename: SAMPLE_CSV_FILENAME,
        data_base64: encodeBase64(utf8Bytes(SAMPLE_CSV)),
      });

      setSampleRunConversationId(created.id);
      setPhase("starter");

      await client.request(method.CONVERSATION_SEND_MESSAGE, {
        conversation_id: created.id,
        text: trimmedPrompt,
      });

      router.replace(`/conversation/${created.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }, [client, loadModels, prompt, recommendation?.agent_id, router, setPhase, setSampleRunConversationId]);

  const handleDrop = async (event: DragEvent) => {
    event.preventDefault();
    setDragActive(false);
    if (running) return;
    const files = Array.from(event.dataTransfer?.files ?? []).filter((file) =>
      file.name.toLowerCase().endsWith(".csv"),
    );
    if (files.length === 0) return;
    const file = files[0]!;
    try {
      const text = await file.text();
      setPrompt(`Turn the attached ${file.name} into a self-contained HTML report. ${text.slice(0, 80) ? "Summarize the data with tables and key totals." : ""}`);
    } catch {
      setPrompt(`Turn the attached ${file.name} into a self-contained HTML report.`);
    }
  };

  const webDropProps =
    Platform.OS === "web"
      ? ({
          onDragOver: (event: DragEvent) => {
            event.preventDefault();
            setDragActive(true);
          },
          onDragLeave: () => setDragActive(false),
          onDrop: (event: DragEvent) => {
            handleDrop(event);
          },
        } as Record<string, unknown>)
      : {};

  if (readinessLoading) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <ActivityIndicator color={theme.colors.accentBright} />
      </View>
    );
  }

  if (readyCount === 0) {
    return null;
  }

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <TitlebarDragRegion />
      <ScrollView contentContainerStyle={{ padding: theme.spacing[6], alignItems: "center" }}>
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[5] }}>
          <View style={{ gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
              {copy.title}
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, lineHeight: 22 }}>{copy.subtitle}</Text>
          </View>

          <View
            {...webDropProps}
            style={{
              padding: theme.spacing[5],
              borderRadius: theme.radius.xl,
              borderWidth: 2,
              borderStyle: "dashed",
              borderColor: dragActive ? theme.colors.accent : theme.colors.borderAccent,
              backgroundColor: theme.colors.surface1,
              alignItems: "center",
              gap: theme.spacing[2],
            }}
          >
            <Upload color={theme.colors.foregroundMuted} size={28} />
            <Text style={{ color: theme.colors.foregroundMuted, textAlign: "center" }}>
              {dragActive ? copy.dropActive : copy.dropHint}
            </Text>
          </View>

          <View
            style={{
              padding: theme.spacing[4],
              borderRadius: theme.radius.lg,
              borderWidth: 1,
              borderColor: theme.colors.border,
              backgroundColor: theme.colors.surface2,
              gap: theme.spacing[2],
            }}
          >
            <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
              <FileSpreadsheet color={theme.colors.accentBright} size={18} />
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{copy.sampleTitle}</Text>
            </View>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>{copy.sampleHint}</Text>
            <Text
              style={{
                color: theme.colors.foregroundMuted,
                fontSize: theme.fontSize.xs,
                fontFamily: theme.fontFamily.mono,
                lineHeight: 18,
              }}
              numberOfLines={4}
            >
              {SAMPLE_CSV}
            </Text>
          </View>

          <View style={{ gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "700" }}>
              {copy.promptLabel.toUpperCase()}
            </Text>
            <TextInput
              value={prompt}
              onChangeText={setPrompt}
              multiline
              editable={!running}
              style={{
                minHeight: 100,
                borderWidth: 1,
                borderColor: theme.colors.border,
                borderRadius: theme.radius.lg,
                padding: theme.spacing[3],
                color: theme.colors.foreground,
                backgroundColor: theme.colors.surface1,
                fontSize: theme.fontSize.sm,
                lineHeight: 22,
                ...(Platform.OS === "web"
                  ? ({ outlineStyle: "none", outlineWidth: 0 } as Record<string, unknown>)
                  : {}),
              }}
            />
          </View>

          {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

          <Button
            label={running ? copy.runningLabel : copy.runSampleLabel}
            onPress={() => void runSample()}
            disabled={running || !prompt.trim()}
          />

          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
            {copy.attachOwnHint}
          </Text>
        </View>
      </ScrollView>
    </View>
  );
}
