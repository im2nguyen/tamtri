import { useRouter } from "expo-router";
import { useEffect, useState } from "react";
import { Modal, Pressable, ScrollView, Text, TextInput, View } from "react-native";
import { method, type OrchestrationRunDto } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { useRecipes } from "@/hooks/use-recipes";
import { useDaemon } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

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
  const router = useRouter();
  const { client } = useDaemon();
  const { recipes, loading, enabled } = useRecipes();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [inputsJson, setInputsJson] = useState("{}");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<OrchestrationRunDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!visible) {
      setSelectedId(null);
      setInputsJson("{}");
      setRunning(false);
      setResult(null);
      setError(null);
      return;
    }
    if (recipes.length > 0 && !selectedId) {
      const first = recipes[0]!;
      setSelectedId(first.id);
      setInputsJson(INPUT_TEMPLATES[first.id] ?? "{}");
    }
  }, [visible, recipes, selectedId]);

  const selectRecipe = (id: string) => {
    setSelectedId(id);
    setInputsJson(INPUT_TEMPLATES[id] ?? "{}");
    setResult(null);
    setError(null);
  };

  const run = async () => {
    if (!selectedId) return;
    setRunning(true);
    setError(null);
    try {
      JSON.parse(inputsJson);
      const dto = await client.request<OrchestrationRunDto>(method.ORCHESTRATION_RUN, {
        recipe_id: selectedId,
        source_conversation_id: conversationId,
        inputs_json: inputsJson,
      });
      setResult(dto);
      onComplete?.(dto);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
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
            Executes synchronously on the daemon. Forked conversations appear when each step completes.
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
            style={{
              minHeight: 140,
              borderWidth: 1,
              borderColor: theme.colors.border,
              borderRadius: theme.radius.md,
              padding: theme.spacing[3],
              color: theme.colors.foreground,
              fontFamily: "monospace",
              fontSize: theme.fontSize.xs,
              backgroundColor: theme.colors.surface0,
            }}
          />

          {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}

          {result ? (
            <View
              style={{
                gap: theme.spacing[2],
                padding: theme.spacing[3],
                backgroundColor: theme.colors.surface2,
                borderRadius: theme.radius.lg,
              }}
            >
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>Run {result.status}</Text>
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                Latest conversation: {result.latest_conversation_id}
              </Text>
              {result.branch_conversation_ids?.length ? (
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                  Branches: {result.branch_conversation_ids.join(", ")}
                </Text>
              ) : null}
              <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2], marginTop: theme.spacing[2] }}>
                <Button
                  label="Open latest"
                  variant="secondary"
                  compact
                  onPress={() => {
                    onClose();
                    router.push(`/conversation/${result.latest_conversation_id}`);
                  }}
                />
                {result.branch_conversation_ids?.map((id) => (
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
              </View>
            </View>
          ) : null}

          <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[3] }}>
            <Button label="Close" variant="ghost" onPress={onClose} />
            <Button label={running ? "Running…" : "Run"} disabled={!selectedId || running} onPress={() => void run()} />
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}
