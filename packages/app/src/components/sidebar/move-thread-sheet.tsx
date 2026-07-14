import { Modal, Pressable, ScrollView, Text, View } from "react-native";
import type { ProjectDto } from "@tamtri/protocol";

import { Button } from "@/components/ui/button";
import { UNFILED_PROJECT_ID } from "@/lib/project-tree";
import { useTheme } from "@/styles/use-theme";

interface MoveThreadSheetProps {
  visible: boolean;
  projects: ProjectDto[];
  currentProjectId?: string | null;
  onClose: () => void;
  onSelect: (projectId: string) => void;
}

export function MoveThreadSheet({
  visible,
  projects,
  currentProjectId,
  onClose,
  onSelect,
}: MoveThreadSheetProps) {
  const theme = useTheme();
  const targets = projects.filter((project) => project.id !== currentProjectId);

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <Pressable
        onPress={onClose}
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
            maxHeight: "70%",
            borderRadius: theme.radius.xl,
            backgroundColor: theme.colors.surface1,
            borderWidth: 1,
            borderColor: theme.colors.border,
            padding: theme.spacing[4],
            gap: theme.spacing[3],
          }}
        >
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "600" }}>
            Move to project
          </Text>
          <ScrollView style={{ maxHeight: 320 }}>
            <View style={{ gap: theme.spacing[2] }}>
              {targets.map((project) => (
                <Pressable
                  key={project.id}
                  onPress={() => onSelect(project.id)}
                  style={({ pressed }) => ({
                    paddingHorizontal: theme.spacing[3],
                    paddingVertical: theme.spacing[3],
                    borderRadius: theme.radius.lg,
                    backgroundColor: pressed ? theme.colors.surface2 : theme.colors.surface0,
                    borderWidth: 1,
                    borderColor: theme.colors.border,
                  })}
                >
                  <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>
                    {project.id === UNFILED_PROJECT_ID ? "Unfiled" : project.name}
                  </Text>
                </Pressable>
              ))}
            </View>
          </ScrollView>
          <Button label="Cancel" variant="secondary" onPress={onClose} />
        </Pressable>
      </Pressable>
    </Modal>
  );
}
