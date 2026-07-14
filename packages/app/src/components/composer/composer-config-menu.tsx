import { Check, ChevronDown, ShieldCheck } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ActivityIndicator,
  Modal,
  Platform,
  Pressable,
  ScrollView,
  Text,
  View,
} from "react-native";

import { shortModelLabel } from "@/components/composer/composer-chip";
import { Button } from "@/components/ui/button";
import { useAgents, type AgentRosterEntry, type ModelEntry } from "@/hooks/use-agents";
import { useReadiness } from "@/hooks/use-readiness";
import { buildConfigTriggerLabel } from "@/lib/composer-config-label";
import { frostedPopupStyle } from "@/styles/surface-styles";
import { useTheme } from "@/styles/use-theme";

interface ComposerConfigMenuProps {
  harnessId?: string;
  harnessDisplayName?: string;
  modelId?: string;
  modelDisplayName?: string;
  activeMode?: string | null;
  runtimeModelSwitch?: boolean;
  controlsDisabled?: boolean;
  onHarnessSelect?: (harnessId: string) => void;
  onModelSelect?: (modelId: string) => Promise<void>;
  onForkRequest?: () => void;
}

function MenuSectionHeader({ label }: { label: string }) {
  const theme = useTheme();
  return (
    <Text
      style={{
        color: theme.colors.foregroundMuted,
        fontSize: theme.fontSize.xs,
        fontWeight: "600",
        letterSpacing: 0.4,
        paddingHorizontal: theme.spacing[2],
        paddingTop: theme.spacing[2],
        paddingBottom: theme.spacing[1],
      }}
    >
      {label}
    </Text>
  );
}

function MenuRadioRow({
  label,
  selected,
  disabled,
  onPress,
  testID,
}: {
  label: string;
  selected: boolean;
  disabled?: boolean;
  onPress?: () => void;
  testID?: string;
}) {
  const theme = useTheme();
  const interactive = Boolean(onPress) && !disabled;

  return (
    <Pressable
      testID={testID}
      onPress={onPress}
      disabled={!interactive}
      style={({ pressed }) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[2],
        minHeight: 40,
        paddingHorizontal: theme.spacing[2],
        paddingVertical: theme.spacing[2],
        borderRadius: theme.radius.lg,
        backgroundColor: pressed && interactive ? theme.colors.surface2 : "transparent",
        opacity: disabled && !selected ? 0.55 : 1,
      })}
    >
      <View style={{ width: 18, alignItems: "center" }}>
        {selected ? <Check color={theme.colors.accent} size={14} strokeWidth={2.5} /> : null}
      </View>
      <Text
        numberOfLines={1}
        style={{
          color: selected ? theme.colors.foreground : theme.colors.foregroundMuted,
          fontSize: theme.fontSize.sm,
          fontWeight: selected ? "600" : "400",
          flex: 1,
        }}
      >
        {label}
      </Text>
    </Pressable>
  );
}

function SectionNote({ children }: { children: string }) {
  const theme = useTheme();
  return (
    <Text
      style={{
        color: theme.colors.foregroundMuted,
        fontSize: theme.fontSize.xs,
        lineHeight: 18,
        paddingHorizontal: theme.spacing[2],
        paddingBottom: theme.spacing[1],
      }}
    >
      {children}
    </Text>
  );
}

export function ComposerPermissionChip() {
  const theme = useTheme();
  const [open, setOpen] = useState(false);

  return (
    <>
      <Pressable
        testID="composer-permissions-chip"
        onPress={() => setOpen(true)}
        style={({ pressed }) => ({
          height: 28,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[1],
          paddingHorizontal: theme.spacing[2],
          borderRadius: theme.radius.full,
          backgroundColor: pressed ? theme.colors.surface3 : theme.colors.surface2,
        })}
      >
        <ShieldCheck color={theme.colors.foregroundMuted} size={12} />
        <Text
          numberOfLines={1}
          style={{
            color: theme.colors.foregroundMuted,
            fontSize: theme.fontSize.xs,
            fontWeight: "500",
          }}
        >
          Ask each time
        </Text>
      </Pressable>

      <Modal visible={open} transparent animationType="fade" onRequestClose={() => setOpen(false)}>
        <Pressable
          onPress={() => setOpen(false)}
          style={{
            flex: 1,
            backgroundColor: "rgba(0,0,0,0.45)",
            justifyContent: "center",
            padding: theme.spacing[4],
          }}
        >
          <Pressable
            onPress={(event) => event.stopPropagation()}
            style={{
              alignSelf: "center",
              width: "100%",
              maxWidth: 400,
              backgroundColor: theme.colors.surface1,
              borderRadius: theme.radius.xl,
              borderWidth: 1,
              borderColor: theme.colors.border,
              padding: theme.spacing[4],
              gap: theme.spacing[2],
            }}
          >
            <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "700" }}>
              Permission policy
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
              tamtri asks before file edits, shell commands, and gateway tool calls. Choose allow once, for this
              conversation, or for this folder on each consent card.
            </Text>
            <View style={{ alignItems: "flex-end" }}>
              <Button label="Close" variant="ghost" onPress={() => setOpen(false)} />
            </View>
          </Pressable>
        </Pressable>
      </Modal>
    </>
  );
}

export function ComposerConfigMenu({
  harnessId,
  harnessDisplayName,
  modelId,
  modelDisplayName,
  activeMode,
  runtimeModelSwitch,
  controlsDisabled,
  onHarnessSelect,
  onModelSelect,
  onForkRequest,
}: ComposerConfigMenuProps) {
  const theme = useTheme();
  const [open, setOpen] = useState(false);
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelSaving, setModelSaving] = useState(false);

  const { agents, loading: agentsLoading, loadModels } = useAgents();
  const { readyEntries, readyCount, loading: readinessLoading } = useReadiness();

  const pickableAgents = useMemo(() => {
    if (readyCount > 0) {
      return agents.filter((agent) => readyEntries.some((entry) => entry.id === agent.id));
    }
    return agents;
  }, [agents, readyCount, readyEntries]);

  const harnessLabel = harnessDisplayName ?? harnessId ?? "Agent app";
  const modelLabel = modelDisplayName ?? (modelId ? shortModelLabel(modelId) : "Model");
  const triggerLabel = buildConfigTriggerLabel(harnessLabel, modelLabel, activeMode);

  const canPickHarness = Boolean(onHarnessSelect && !controlsDisabled);
  const canPickModel = Boolean(onModelSelect && runtimeModelSwitch && !controlsDisabled);

  useEffect(() => {
    if (!open || !harnessId) {
      setModels([]);
      return;
    }
    setModelsLoading(true);
    void loadModels(harnessId)
      .then(setModels)
      .catch(() => setModels([]))
      .finally(() => setModelsLoading(false));
  }, [harnessId, loadModels, open]);

  const close = useCallback(() => setOpen(false), []);

  const handleHarnessSelect = useCallback(
    (agent: AgentRosterEntry) => {
      if (!canPickHarness || agent.id === harnessId) return;
      onHarnessSelect?.(agent.id);
      close();
    },
    [canPickHarness, close, harnessId, onHarnessSelect],
  );

  const handleModelSelect = useCallback(
    async (model: ModelEntry) => {
      if (!canPickModel || !onModelSelect || model.id === modelId || modelSaving) return;
      setModelSaving(true);
      try {
        await onModelSelect(model.id);
        close();
      } finally {
        setModelSaving(false);
      }
    },
    [canPickModel, close, modelId, modelSaving, onModelSelect],
  );

  const currentModelRow =
    models.find((model) => model.id === modelId) ??
    (modelId ? { id: modelId, display_name: modelLabel } : null);

  const rosterLoading = agentsLoading || readinessLoading;

  return (
    <>
      <Pressable
        testID="composer-config-trigger"
        onPress={() => setOpen(true)}
        disabled={controlsDisabled}
        accessibilityRole="button"
        accessibilityLabel={`Composer settings: ${triggerLabel}`}
        style={({ pressed }) => ({
          height: 28,
          maxWidth: 220,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[1],
          paddingHorizontal: theme.spacing[2],
          borderRadius: theme.radius.full,
          backgroundColor: pressed ? theme.colors.surface3 : theme.colors.surface2,
          opacity: controlsDisabled ? 0.55 : 1,
        })}
      >
        <Text
          numberOfLines={1}
          style={{
            color: theme.colors.foregroundMuted,
            fontSize: theme.fontSize.xs,
            fontWeight: "500",
            flexShrink: 1,
          }}
        >
          {triggerLabel}
        </Text>
        <ChevronDown color={theme.colors.foregroundMuted} size={12} />
      </Pressable>

      <Modal visible={open} transparent animationType="fade" onRequestClose={close}>
        <Pressable
          onPress={close}
          style={{
            flex: 1,
            backgroundColor: "rgba(0,0,0,0.45)",
            justifyContent: Platform.OS === "web" ? "center" : "flex-end",
            padding: theme.spacing[4],
          }}
        >
          <Pressable
            onPress={(event) => event.stopPropagation()}
            style={[
              frostedPopupStyle(theme),
              {
                alignSelf: Platform.OS === "web" ? "center" : "stretch",
                width: Platform.OS === "web" ? 320 : "100%",
                maxHeight: Platform.OS === "web" ? 480 : "70%",
                paddingVertical: theme.spacing[2],
              },
            ]}
          >
            <ScrollView bounces={false} contentContainerStyle={{ paddingBottom: theme.spacing[2] }}>
              <MenuSectionHeader label="AGENT" />
              {rosterLoading ? (
                <View style={{ paddingHorizontal: theme.spacing[2], paddingVertical: theme.spacing[2] }}>
                  <ActivityIndicator color={theme.colors.foregroundMuted} size="small" />
                </View>
              ) : pickableAgents.length === 0 ? (
                <SectionNote>No agent apps are ready. Open Settings → Agents & providers to set one up.</SectionNote>
              ) : canPickHarness ? (
                pickableAgents.map((agent) => (
                  <MenuRadioRow
                    key={agent.id}
                    testID={`composer-agent-option-${agent.id}`}
                    label={agent.display_name}
                    selected={agent.id === harnessId}
                    onPress={() => handleHarnessSelect(agent)}
                  />
                ))
              ) : (
                <>
                  <MenuRadioRow
                    testID="composer-agent-current"
                    label={harnessLabel}
                    selected
                    disabled
                  />
                  <SectionNote>
                    Fixed for this thread. Agent app and model are chosen when you create or fork a conversation.
                  </SectionNote>
                  {onForkRequest ? (
                    <View style={{ paddingHorizontal: theme.spacing[2], paddingTop: theme.spacing[1] }}>
                      <Button label="Fork into…" variant="ghost" compact onPress={() => { close(); onForkRequest(); }} />
                    </View>
                  ) : null}
                </>
              )}

              <View
                style={{
                  height: 1,
                  backgroundColor: theme.colors.border,
                  marginHorizontal: theme.spacing[2],
                  marginVertical: theme.spacing[2],
                }}
              />

              <MenuSectionHeader label="MODEL" />
              {modelsLoading ? (
                <View style={{ paddingHorizontal: theme.spacing[2], paddingVertical: theme.spacing[2] }}>
                  <ActivityIndicator color={theme.colors.foregroundMuted} size="small" />
                </View>
              ) : canPickModel ? (
                models.length === 0 && currentModelRow ? (
                  <MenuRadioRow
                    testID="composer-model-current"
                    label={currentModelRow.display_name}
                    selected
                    onPress={() => {}}
                  />
                ) : (
                  models.map((model) => (
                    <MenuRadioRow
                      key={model.id}
                      testID={`composer-model-option-${model.id}`}
                      label={model.display_name}
                      selected={model.id === modelId}
                      disabled={modelSaving}
                      onPress={() => void handleModelSelect(model)}
                    />
                  ))
                )
              ) : (
                <>
                  <MenuRadioRow
                    testID="composer-model-current"
                    label={modelLabel}
                    selected
                    disabled
                  />
                  <SectionNote>
                    {runtimeModelSwitch
                      ? "Model selection is unavailable while the conversation is busy."
                      : `${modelLabel} is fixed for this thread. Fork to try a different model or agent app.`}
                  </SectionNote>
                  {!runtimeModelSwitch && onForkRequest ? (
                    <View style={{ paddingHorizontal: theme.spacing[2], paddingTop: theme.spacing[1] }}>
                      <Button label="Fork into…" variant="ghost" compact onPress={() => { close(); onForkRequest(); }} />
                    </View>
                  ) : null}
                </>
              )}

              <View
                style={{
                  height: 1,
                  backgroundColor: theme.colors.border,
                  marginHorizontal: theme.spacing[2],
                  marginVertical: theme.spacing[2],
                }}
              />

              <MenuSectionHeader label="PERMISSION" />
              <MenuRadioRow testID="composer-permission-ask" label="Ask each time" selected disabled />
              <SectionNote>
                Choose allow once, for this conversation, or for this folder on each consent card.
              </SectionNote>
            </ScrollView>
          </Pressable>
        </Pressable>
      </Modal>
    </>
  );
}
