import { useRouter } from "expo-router";
import { PanelLeft } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ActivityIndicator, Pressable, Text, useWindowDimensions, View } from "react-native";
import { method } from "@tamtri/protocol";

import { Composer } from "@/components/composer/composer";
import { shortModelLabel } from "@/components/composer/composer-chip";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { Button } from "@/components/ui/button";
import { isCompact, MAX_CONTENT_WIDTH } from "@/constants/layout";
import { onboardingCopy } from "@/content/onboarding-copy";
import { useAgents } from "@/hooks/use-agents";
import { useConversationList } from "@/hooks/use-conversations";
import { useProjects } from "@/hooks/use-projects";
import { useReadiness } from "@/hooks/use-readiness";
import { encodeBase64 } from "@/lib/base64";
import { UNFILED_PROJECT_ID } from "@/lib/project-tree";
import { resolveHomeRestoration } from "@/lib/home-restoration";
import { electronFilePath, shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

export function HomePane() {
  const theme = useTheme();
  const router = useRouter();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const { client } = useDaemon();
  const {
    conversations,
    loading: conversationsLoading,
    refresh: refreshConversations,
  } = useConversationList();
  const {
    projects,
    projectsSupported,
    loading: projectsLoading,
    createConversation,
  } = useProjects();
  const { agents, loadModels } = useAgents();
  const { recommendation, readyCount, loading: readinessLoading } = useReadiness();
  const selectedProjectId = useUiStore((s) => s.selectedProjectId);
  const draftProjectId = useUiStore((s) => s.draftProjectId);
  const setSelectedProject = useUiStore((s) => s.setSelectedProject);
  const clearProjectDraft = useUiStore((s) => s.clearProjectDraft);
  const restorationAttempted = useRef(false);
  const [sending, setSending] = useState(false);
  const [attaching, setAttaching] = useState(false);
  const [pendingFiles, setPendingFiles] = useState<File[]>([]);
  const [pendingPaths, setPendingPaths] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedHarnessId, setSelectedHarnessId] = useState<string | undefined>();
  const [selectedModelId, setSelectedModelId] = useState<string | undefined>();
  const [modelDisplayName, setModelDisplayName] = useState<string | undefined>();
  const copy = onboardingCopy.home;

  useEffect(() => {
    if (readinessLoading) return;
    const recommended = recommendation?.agent_id;
    if (!recommended) return;
    setSelectedHarnessId((current) => current ?? recommended);
  }, [readinessLoading, recommendation?.agent_id]);

  useEffect(() => {
    if (!selectedHarnessId) {
      setSelectedModelId(undefined);
      setModelDisplayName(undefined);
      return;
    }
    void loadModels(selectedHarnessId)
      .then((models) => {
        const first = models[0];
        setSelectedModelId((current) => current ?? first?.id ?? "default");
        setModelDisplayName((current) => {
          if (current) return current;
          return first?.display_name ?? (first?.id ? shortModelLabel(first.id) : "Default");
        });
      })
      .catch(() => {
        setSelectedModelId((current) => current ?? "default");
        setModelDisplayName((current) => current ?? "Default");
      });
  }, [loadModels, selectedHarnessId]);

  useEffect(() => {
    if (!selectedHarnessId || !selectedModelId) return;
    void loadModels(selectedHarnessId)
      .then((models) => {
        const match = models.find((model) => model.id === selectedModelId);
        setModelDisplayName(match?.display_name ?? shortModelLabel(selectedModelId));
      })
      .catch(() => setModelDisplayName(shortModelLabel(selectedModelId)));
  }, [loadModels, selectedHarnessId, selectedModelId]);

  const harnessDisplayName = useMemo(() => {
    if (!selectedHarnessId) return recommendation?.display_name;
    return agents.find((agent) => agent.id === selectedHarnessId)?.display_name ?? selectedHarnessId;
  }, [agents, recommendation?.display_name, selectedHarnessId]);

  const attachedNames = useMemo(
    () => [
      ...pendingFiles.map((file) => file.name),
      ...pendingPaths.map((path) => path.split("/").pop() ?? path),
    ],
    [pendingFiles, pendingPaths],
  );

  const selectedProject =
    projects.find((project) => project.id === selectedProjectId) ?? null;

  useEffect(() => {
    if (projectsLoading || conversationsLoading || restorationAttempted.current) return;
    restorationAttempted.current = true;
    const validProjectIds = new Set(projects.map((project) => project.id));
    const decision = resolveHomeRestoration({
      draftProjectId,
      validProjectIds,
      conversations,
    });
    if (decision.action === "stay") {
      if (decision.projectId && selectedProjectId !== decision.projectId) {
        setSelectedProject(decision.projectId);
      }
      const firstProject = projects.find(
        (project) => project.id !== UNFILED_PROJECT_ID,
      );
      if (!selectedProject && firstProject) {
        setSelectedProject(firstProject.id);
      }
      return;
    }

    if (decision.projectId) {
      setSelectedProject(decision.projectId);
    }
    router.replace(`/conversation/${decision.conversationId}`);
  }, [
    conversations,
    conversationsLoading,
    draftProjectId,
    projects,
    projectsLoading,
    router,
    selectedProject,
    selectedProjectId,
    setSelectedProject,
  ]);

  useEffect(() => {
    if (projectsLoading || !selectedProjectId || selectedProject) return;
    const firstProject =
      projects.find((project) => project.id !== UNFILED_PROJECT_ID) ?? null;
    clearProjectDraft();
    setSelectedProject(firstProject?.id ?? null);
  }, [
    clearProjectDraft,
    projects,
    projectsLoading,
    selectedProject,
    selectedProjectId,
    setSelectedProject,
  ]);

  const handleHarnessSelect = useCallback((harnessId: string) => {
    setSelectedHarnessId(harnessId);
    setSelectedModelId(undefined);
    setModelDisplayName(undefined);
  }, []);

  const handleModelSelect = useCallback(async (modelId: string) => {
    setSelectedModelId(modelId);
  }, []);

  const attachFilesToConversation = useCallback(
    async (conversationId: string, files: File[], paths: string[]) => {
      if (files.length === 0 && paths.length === 0) return;
      setAttaching(true);
      try {
        await Promise.all(
          paths.map((sourcePath) =>
            client.request(method.WORKDIR_COPY_FILE, {
              conversation_id: conversationId,
              source_path: sourcePath,
            }),
          ),
        );
        await Promise.all(
          files.map(async (file) => {
            const electronPath = electronFilePath(file);
            if (electronPath) {
              await client.request(method.WORKDIR_COPY_FILE, {
                conversation_id: conversationId,
                source_path: electronPath,
              });
              return;
            }
            const buffer = new Uint8Array(await file.arrayBuffer());
            await client.request(method.WORKDIR_WRITE_FILE, {
              conversation_id: conversationId,
              filename: file.name,
              data_base64: encodeBase64(buffer),
            });
          }),
        );
      } finally {
        setAttaching(false);
      }
    },
    [client],
  );

  const handleSend = useCallback(
    async (text: string) => {
      const harnessId = selectedHarnessId ?? recommendation?.agent_id;
      const modelId = selectedModelId ?? "default";
      if (!harnessId) {
        setError(copy.noAgentsBody);
        return;
      }

      setSending(true);
      setError(null);
      try {
        if (!selectedProject) {
          setError("Create or select a project before starting a thread.");
          return;
        }
        const created = await createConversation(
          selectedProject.id,
          text.slice(0, 80) || "New thread",
          harnessId,
          modelId,
        );

        const files = pendingFiles;
        const paths = pendingPaths;
        setPendingFiles([]);
        setPendingPaths([]);
        await attachFilesToConversation(created.id, files, paths);

        await client.request(method.CONVERSATION_SEND_MESSAGE, {
          conversation_id: created.id,
          text,
        });

        await refreshConversations();
        clearProjectDraft();
        router.push(`/conversation/${created.id}`);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setSending(false);
      }
    },
    [
      attachFilesToConversation,
      clearProjectDraft,
      client,
      copy.noAgentsBody,
      createConversation,
      pendingFiles,
      pendingPaths,
      recommendation?.agent_id,
      refreshConversations,
      router,
      selectedProject,
      selectedHarnessId,
      selectedModelId,
    ],
  );

  const handlePickFile = useCallback(async () => {
    const shell = shellBridge();
    if (shell?.pickOpenFile) {
      const path = await shell.pickOpenFile({ title: "Attach to conversation" });
      if (path) {
        setPendingPaths((current) => [...current, path]);
      }
    }
  }, []);

  const handleDropFiles = useCallback(async (files: File[]) => {
    setPendingFiles((current) => [...current, ...files]);
  }, []);

  if (readinessLoading || projectsLoading || conversationsLoading) {
    return (
      <View
        style={{
          flex: 1,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: theme.colors.surface0,
        }}
      >
        <ActivityIndicator color={theme.colors.accentBright} />
      </View>
    );
  }

  return (
    <View
      style={{
        flex: 1,
        backgroundColor: theme.colors.surface0,
        alignItems: "center",
        justifyContent: "center",
        padding: theme.spacing[6],
      }}
    >
      <TitlebarDragRegion />
      {compact ? (
        <Pressable onPress={toggleSidebar} style={{ position: "absolute", top: 16, left: 16 }}>
          <PanelLeft color={theme.colors.foreground} size={20} />
        </Pressable>
      ) : null}

      <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[5] }}>
        <View style={{ gap: theme.spacing[2], alignItems: "center" }}>
          <Text
            style={{
              color: theme.colors.foreground,
              fontSize: 24,
              fontWeight: "600",
              textAlign: "center",
            }}
          >
            New thread
          </Text>
          <Text
            style={{
              color: theme.colors.foregroundMuted,
              fontSize: theme.fontSize.base,
              textAlign: "center",
              lineHeight: 24,
              maxWidth: 520,
            }}
          >
            {selectedProject
              ? `Start a conversation in ${selectedProject.name}`
              : "Create or select a project from the sidebar to start a thread."}
          </Text>
        </View>

        {!projectsSupported ? (
          <Text accessibilityRole="alert" style={{ color: theme.colors.foregroundMuted, textAlign: "center" }}>
            Update the host to use projects.
          </Text>
        ) : !selectedProject ? null : readyCount === 0 ? (
          <View
            style={{
              padding: theme.spacing[4],
              borderRadius: theme.radius.lg,
              borderWidth: 1,
              borderColor: theme.colors.border,
              backgroundColor: theme.colors.surface1,
              gap: theme.spacing[3],
              alignItems: "center",
            }}
          >
            <Text style={{ color: theme.colors.foreground, fontWeight: "600", textAlign: "center" }}>
              {copy.noAgentsTitle}
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, textAlign: "center", lineHeight: 22 }}>
              {copy.noAgentsBody}
            </Text>
            <Button label={copy.setupAgentsLabel} onPress={() => router.push("/settings/providers")} />
          </View>
        ) : (
          <Composer
            layout="inline"
            onSend={handleSend}
            sending={sending}
            attaching={attaching}
            placeholder={copy.composerPlaceholder}
            attachedFiles={attachedNames}
            onPickFile={shellBridge()?.pickOpenFile ? () => void handlePickFile() : undefined}
            onDropFiles={handleDropFiles}
            harnessId={selectedHarnessId}
            harnessDisplayName={harnessDisplayName}
            modelId={selectedModelId}
            modelDisplayName={modelDisplayName}
            runtimeModelSwitch
            onHarnessSelect={handleHarnessSelect}
            onModelSelect={handleModelSelect}
          />
        )}

        {error ? (
          <Text style={{ color: theme.colors.destructive, textAlign: "center" }}>{error}</Text>
        ) : null}

      </View>

    </View>
  );
}
