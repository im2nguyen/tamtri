import { useRouter } from "expo-router";
import { useEffect, useState } from "react";
import { ActivityIndicator, Modal, Pressable, ScrollView, Text, TextInput, View } from "react-native";
import { method, type OrchestrationRunDto } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { useOrchestrationRun } from "@/hooks/use-orchestration-run";
import { useRecipes } from "@/hooks/use-recipes";
import { useDaemon } from "@/runtime/daemon-provider";
import { useTheme } from "@/styles/use-theme";

const INPUT_TEMPLATES: Record<string, string> = {
  handoff: JSON.stringify(
    {
      harness_id: "claude-native",
      model_id: "default",
      message: "Continue this task with full context from the parent conversation.",
    },
    null,
    2,
  ),
  committee: JSON.stringify(
    {
      harness_a: "claude-native",
      model_a: "default",
      harness_b: "codex-native",
      model_b: "default",
      prompt: "Review this plan. Analysis only — do not edit files.",
    },
    null,
    2,
  ),
};

interface RunRecipeSheetProps {
  visible: boolean;
  conversationId: string;
  onClose: () => void;
  onComplete?: (result: OrchestrationRunDto) => void;
}

function SelectRow({
  label,
  selected,
  onPress,
}: {
  label: string;
  selected: boolean;
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
            : theme.colors.surface1,
        borderWidth: 1,
        borderColor: selected ? theme.colors.accent : theme.colors.border,
      })}
    >
      <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>{label}</Text>
    </Pressable>
  );
}

export function RunRecipeSheet({ visible, conversationId, onClose, onComplete }: RunRecipeSheetProps) {
  const theme = useTheme();
  const router = useRouter();
  const { client } = useDaemon();
  const { recipes, loading, enabled } = useRecipes();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [inputsJson, setInputsJson] = useState("{}");
  const [starting, setStarting] = useState(false);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { run, isRunning, cancel, refresh, branchConversationIds } = useOrchestrationRun(conversationId, activeRunId);

  useEffect(() => {
    if (!visible) {
      setSelectedId(null);
      setInputsJson("{}");
      setStarting(false);
      setActiveRunId(null);
      setError(null);
      return;
    }
    if (recipes.length > 0 && !selectedId) {
      const first = recipes[0]!;
      setSelectedId(first.id);
      setInputsJson(INPUT_TEMPLATES[first.id] ?? "{}");
    }
  }, [visible, recipes, selectedId]);

  useEffect(() => {
    if (run && !isRunning) {
      onComplete?.(run);
    }
  }, [isRunning, onComplete, run]);

  const selectRecipe = (id: string) => {
    setSelectedId(id);
    setInputsJson(INPUT_TEMPLATES[id] ?? "{}");
    setActiveRunId(null);
    setError(null);
  };

  const startRun = async () => {
    if (!selectedId) return;
    setStarting(true);
    setError(null);
    try {
      JSON.parse(inputsJson);
      const dto = await client.request<OrchestrationRunDto>(method.ORCHESTRATION_RUN, {
        recipe_id: selectedId,
        source_conversation_id: conversationId,
        inputs_json: inputsJson,
      });
      setActiveRunId(dto.id);
      if (dto.status !== "running") {
        onComplete?.(dto);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStarting(false);
    }
  };

  if (!enabled) {
    return (
      <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
        <Pressable
          onPress={onClose}
          style={{ flex: 1, backgroundColor: "rgba(0,0,0,0.55)", justifyContent: "center", padding: theme.spacing[6] }}
        >
          <View
            style={{
              alignSelf: "center",
              maxWidth: 420,
              backgroundColor: theme.colors.surface1,
              borderRadius: theme.radius.xl,
              padding: theme.spacing[5],
              gap: theme.spacing[3],
            }}
          >
            <Text style={{ color: theme.colors.foreground }}>Orchestration is not available on this host.</Text>
            <Button label="Close" variant="ghost" onPress={onClose} />
          </View>
        </Pressable>
      </Modal>
    );
  }

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <Pressable
        onPress={onClose}
        style={{ flex: 1, backgroundColor: "rgba(0,0,0,0.55)", justifyContent: "center", padding: theme.spacing[6] }}
      >
        <Pressable
          onPress={(event) => event.stopPropagation()}
          style={{
            alignSelf: "center",
            width: "100%",
            maxWidth: 520,
            maxHeight: "90%",
            backgroundColor: theme.colors.surface1,
            borderRadius: theme.radius.xl,
            borderWidth: 1,
            borderColor: theme.colors.border,
            padding: theme.spacing[5],
            gap: theme.spacing[4],
          }}
        >
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
            Run recipe
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
            Runs in the background on the daemon. Subscribe to live step events and open forked conversations as they
            appear.
          </Text>

          {loading ? (
            <Text style={{ color: theme.colors.foregroundMuted }}>Loading recipes…</Text>
          ) : (
            <ScrollView style={{ maxHeight: 120 }} contentContainerStyle={{ gap: theme.spacing[2] }}>
              {recipes.map((recipe) => (
                <SelectRow
                  key={recipe.id}
                  label={recipe.title}
                  selected={selectedId === recipe.id}
                  onPress={() => selectRecipe(recipe.id)}
                />
              ))}
            </ScrollView>
          )}

          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
            INPUTS (JSON)
          </Text>
          <TextInput
            value={inputsJson}
            onChangeText={setInputsJson}
            multiline
            editable={!isRunning}
            style={{
              minHeight: 140,
              borderWidth: 1,
              borderColor: theme.colors.border,
              borderRadius: theme.radius.md,
              padding: theme.spacing[3],
              color: theme.colors.foreground,
              fontFamily: theme.fontFamily.mono,
              fontSize: theme.fontSize.xs,
              backgroundColor: theme.colors.surface0,
            }}
          />

          {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

          {run ? (
            <View
              style={{
                gap: theme.spacing[2],
                padding: theme.spacing[3],
                backgroundColor: theme.colors.surface2,
                borderRadius: theme.radius.lg,
              }}
            >
              <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
                {isRunning ? <ActivityIndicator color={theme.colors.accentBright} size="small" /> : null}
                <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>
                  Run {run.status} · step {run.current_step + 1}
                </Text>
              </View>
              {run.error ? (
                <Text style={{ color: theme.colors.destructive, fontSize: theme.fontSize.xs }}>{run.error}</Text>
              ) : null}
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                Latest conversation: {run.latest_conversation_id}
              </Text>
              <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2], marginTop: theme.spacing[2] }}>
                <Button
                  label="Open latest"
                  variant="secondary"
                  compact
                  onPress={() => {
                    onClose();
                    router.push(`/conversation/${run.latest_conversation_id}`);
                  }}
                />
                {branchConversationIds.map((id) => (
                  <Button
                    key={id}
                    label={`Branch ${id.slice(0, 8)}…`}
                    variant="ghost"
                    compact
                    onPress={() => {
                      onClose();
                      router.push(`/conversation/${id}`);
                    }}
                  />
                ))}
                {isRunning ? (
                  <Button label="Cancel run" variant="destructive" compact onPress={() => void cancel()} />
                ) : (
                  <Button label="Refresh" variant="ghost" compact onPress={() => void refresh()} />
                )}
              </View>
            </View>
          ) : null}

          <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[3] }}>
            <Button label="Close" variant="ghost" onPress={onClose} />
            <Button
              label={starting ? "Starting…" : isRunning ? "Running…" : "Run"}
              disabled={!selectedId || starting || isRunning}
              onPress={() => void startRun()}
            />
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}
