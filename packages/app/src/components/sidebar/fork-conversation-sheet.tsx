import { useEffect, useState } from "react";
import { Modal, Pressable, ScrollView, Text, View } from "react-native";
import { method, type ConversationDto } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { useAgents, type AgentRosterEntry, type ModelEntry } from "@/hooks/use-agents";
import { useDaemon } from "@/runtime/daemon-provider";
import { useTheme } from "@/styles/use-theme";

interface ForkConversationSheetProps {
  visible: boolean;
  conversationId: string;
  sourceMessageId?: string;
  onClose: () => void;
  onForked: (conversation: ConversationDto) => void;
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

export function ForkConversationSheet({
  visible,
  conversationId,
  sourceMessageId,
  onClose,
  onForked,
}: ForkConversationSheetProps) {
  const theme = useTheme();
  const { agents, loading, loadModels } = useAgents();
  const { client } = useDaemon();
  const [selectedAgent, setSelectedAgent] = useState<AgentRosterEntry | null>(null);
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [selectedModel, setSelectedModel] = useState<ModelEntry | null>(null);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [forking, setForking] = useState(false);

  useEffect(() => {
    if (!visible) {
      setSelectedAgent(null);
      setModels([]);
      setSelectedModel(null);
      return;
    }
    if (agents.length > 0 && !selectedAgent) {
      setSelectedAgent(agents[0] ?? null);
    }
  }, [visible, agents, selectedAgent]);

  useEffect(() => {
    if (!selectedAgent) return;
    setModelsLoading(true);
    void loadModels(selectedAgent.id)
      .then((rows) => {
        setModels(rows);
        setSelectedModel(rows[0] ?? { id: "default", display_name: "Default" });
      })
      .catch(() => {
        setModels([]);
        setSelectedModel({ id: "default", display_name: "Default" });
      })
      .finally(() => setModelsLoading(false));
  }, [loadModels, selectedAgent]);

  const submit = async () => {
    if (!selectedAgent || !selectedModel) return;
    setForking(true);
    try {
      const forked = await client.request<ConversationDto>(method.CONVERSATION_FORK, {
        id: conversationId,
        harness_id: selectedAgent.id,
        model_id: selectedModel.id,
      });
      onForked(forked);
      onClose();
    } finally {
      setForking(false);
    }
  };

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
            maxWidth: 480,
            backgroundColor: theme.colors.surface1,
            borderRadius: theme.radius.xl,
            borderWidth: 1,
            borderColor: theme.colors.border,
            padding: theme.spacing[5],
            gap: theme.spacing[4],
          }}
        >
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
            Fork into…
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
            {sourceMessageId
              ? "Copy this conversation's full context into a new thread with a different harness or model. Message-boundary forks are not supported yet — the entire transcript is copied."
              : "Copy this conversation's context into a new thread with a different harness or model."}
          </Text>

          <View style={{ gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
              HARNESS
            </Text>
            {loading ? (
              <Text style={{ color: theme.colors.foregroundMuted }}>Loading agents…</Text>
            ) : (
              <ScrollView style={{ maxHeight: 160 }} contentContainerStyle={{ gap: theme.spacing[2] }}>
                {agents.map((agent) => (
                  <SelectRow
                    key={agent.id}
                    label={agent.display_name}
                    selected={selectedAgent?.id === agent.id}
                    onPress={() => setSelectedAgent(agent)}
                  />
                ))}
              </ScrollView>
            )}
          </View>

          <View style={{ gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
              MODEL
            </Text>
            {modelsLoading ? (
              <Text style={{ color: theme.colors.foregroundMuted }}>Loading models…</Text>
            ) : models.length === 0 ? (
              <SelectRow label="Default" selected onPress={() => {}} />
            ) : (
              <ScrollView style={{ maxHeight: 120 }} contentContainerStyle={{ gap: theme.spacing[2] }}>
                {models.map((model) => (
                  <SelectRow
                    key={model.id}
                    label={model.display_name}
                    selected={selectedModel?.id === model.id}
                    onPress={() => setSelectedModel(model)}
                  />
                ))}
              </ScrollView>
            )}
          </View>

          <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: theme.spacing[3] }}>
            <Button label="Cancel" variant="ghost" onPress={onClose} />
            <Button
              label={forking ? "Forking…" : "Fork"}
              disabled={!selectedAgent || !selectedModel || forking}
              onPress={() => void submit()}
            />
          </View>
        </Pressable>
      </Pressable>
    </Modal>
  );
}
