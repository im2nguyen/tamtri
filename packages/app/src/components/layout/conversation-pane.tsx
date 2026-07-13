import { useRouter } from "expo-router";
import { ArrowDown, PanelLeft, PanelRight, Share2, Workflow } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  Pressable,
  ScrollView,
  Text,
  useWindowDimensions,
  View,
  type NativeScrollEvent,
  type NativeSyntheticEvent,
} from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { method, type ConversationDto } from "@tamtri/protocol";

import { RightDock } from "@/components/artifact/right-dock";
import { Composer } from "@/components/composer/composer";
import { ElicitationCard } from "@/components/consent/elicitation-card";
import { PermissionCard } from "@/components/consent/permission-card";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { ErrorCard } from "@/components/errors/error-card";
import { OnboardingCard } from "@/components/onboarding/onboarding-card";
import { RunRecipeSheet } from "@/components/orchestration/run-recipe-sheet";
import { ForkConversationSheet } from "@/components/sidebar/fork-conversation-sheet";
import { MessageList } from "@/components/transcript/message-list";
import { Button } from "@/components/ui/button";
import { CONVERSATION_COLUMN_WIDTH, isCompact } from "@/constants/layout";
import { onboardingCopy } from "@/content/onboarding-copy";
import { useAgents } from "@/hooks/use-agents";
import { useReadiness } from "@/hooks/use-readiness";
import { useConversation } from "@/hooks/use-conversation";
import { useRoots } from "@/hooks/use-roots";
import { useWorkdir } from "@/hooks/use-workdir";
import { collectArtifactsFromUiMessages } from "@/lib/artifacts";
import { deriveRightDockState, isNearTranscriptBottom } from "@/lib/conversation-surface";
import { isExampleConversation } from "@/lib/conversation-kind";
import { classifyDaemonError } from "@/lib/errors";
import { shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";
import { useOnboardingStore } from "@/stores/onboarding-store";
import { useUiStore } from "@/stores/ui-store";
import { useTheme } from "@/styles/use-theme";

interface ConversationPaneProps {
  conversationId: string;
}

function HeaderAction({
  label,
  icon,
  onPress,
}: {
  label: string;
  icon: ReactNode;
  onPress: () => void;
}) {
  const theme = useTheme();
  return (
    <Pressable
      onPress={onPress}
      accessibilityRole="button"
      accessibilityLabel={label}
      hitSlop={8}
      style={({ pressed }) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[1],
        paddingHorizontal: theme.spacing[2],
        paddingVertical: theme.spacing[1],
        borderRadius: theme.radius.md,
        backgroundColor: pressed ? theme.colors.surface2 : "transparent",
      })}
    >
      {icon}
      <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
        {label}
      </Text>
    </Pressable>
  );
}

export function ConversationPane({ conversationId }: ConversationPaneProps) {
  const theme = useTheme();
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const beginProjectDraft = useUiStore((s) => s.beginProjectDraft);
  const artifactSidebarOpen = useUiStore((s) => s.artifactSidebarOpen);
  const toggleArtifactSidebar = useUiStore((s) => s.toggleArtifactSidebar);
  const closeArtifactPreview = useUiStore((s) => s.closeArtifactPreview);
  const clearArtifactSelection = useUiStore((s) => s.clearArtifactSelection);
  const openArtifactPreview = useUiStore((s) => s.openArtifactPreview);
  const selectedArtifact = useUiStore((s) => s.selectedArtifact);
  const { client, serverInfo } = useDaemon();
  const orchestrationEnabled = Boolean(serverInfo?.features?.orchestration);
  const [forkOpen, setForkOpen] = useState(false);
  const [forkSourceMessageId, setForkSourceMessageId] = useState<string | undefined>();
  const [recipeOpen, setRecipeOpen] = useState(false);
  const [attaching, setAttaching] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [copyingExample, setCopyingExample] = useState(false);
  const autoOpenedFirstArtifact = useRef(false);
  const sampleWorkedAt = useOnboardingStore((s) => s.sample_worked_at);
  const myFileWorkedAt = useOnboardingStore((s) => s.my_file_worked_at);
  const sampleRunConversationId = useOnboardingStore((s) => s.sample_run_conversation_id);
  const onboardingPhase = useOnboardingStore((s) => s.onboarding_phase);
  const exampleDismissed = useOnboardingStore((s) => s.example_dismissed);
  const markSampleWorked = useOnboardingStore((s) => s.markSampleWorked);
  const markMyFileWorked = useOnboardingStore((s) => s.markMyFileWorked);
  const dismissExample = useOnboardingStore((s) => s.dismissExample);
  const completeOnboarding = useOnboardingStore((s) => s.completeOnboarding);
  const { agents, loadModels } = useAgents();
  const { readyCount } = useReadiness();
  const {
    files,
    attachBrowserFiles,
    pickAndAttach,
    canPickFile,
  } = useWorkdir(conversationId);
  const { attachFilesystemRoot, canAttachRoot } = useRoots(conversationId);
  const attachedNames = useMemo(() => files.map((file) => file.relative_path.split("/").pop() ?? file.relative_path), [files]);
  const {
    conversation,
    uiMessages,
    liveUiMessage,
    loading,
    sending,
    isRunning,
    pendingPermission,
    respondingPermission,
    pendingElicitation,
    respondingElicitation,
    activeMode,
    error,
    sendMessage,
    cancelRun,
    respondPermission,
    respondElicitation,
    setModel,
  } = useConversation(conversationId);

  const blockedByConsent = Boolean(pendingPermission || pendingElicitation);
  const classifiedError = error ? classifyDaemonError(error) : null;
  const artifacts = useMemo(() => collectArtifactsFromUiMessages(uiMessages), [uiMessages]);
  const dockState = useMemo(
    () => deriveRightDockState(uiMessages, artifacts.length),
    [artifacts.length, uiMessages],
  );
  const transcriptRef = useRef<ScrollView>(null);
  const followLatestRef = useRef(true);
  const programmaticScrollUntilRef = useRef(0);
  const [showScrollToBottom, setShowScrollToBottom] = useState(false);
  const isExample = isExampleConversation({
    title: conversation?.title,
    kind: conversation?.kind,
  });
  const showExampleCard = isExample && !exampleDismissed;
  const showDropYoursCard =
    onboardingPhase !== "complete" && Boolean(sampleWorkedAt) && !myFileWorkedAt;

  useEffect(() => {
    if (artifacts.length === 0) return;
    if (sampleRunConversationId === conversationId && !sampleWorkedAt) {
      markSampleWorked();
    }
  }, [artifacts.length, conversationId, markSampleWorked, sampleRunConversationId, sampleWorkedAt]);

  useEffect(() => {
    if (!sampleWorkedAt || myFileWorkedAt || attachedNames.length === 0) return;
    const hasOwnFile = attachedNames.some((name) => name !== "sales.csv");
    if (hasOwnFile && artifacts.length > 0) {
      markMyFileWorked();
      completeOnboarding();
    }
  }, [
    artifacts.length,
    attachedNames,
    completeOnboarding,
    markMyFileWorked,
    myFileWorkedAt,
    sampleWorkedAt,
  ]);

  const harnessDisplayName = useMemo(() => {
    const id = conversation?.active_harness_id;
    if (!id) return undefined;
    return agents.find((agent) => agent.id === id)?.display_name ?? id;
  }, [agents, conversation?.active_harness_id]);

  const runtimeModelSwitch = useMemo(() => {
    const id = conversation?.active_harness_id;
    if (!id || isExample) return false;
    const agent = agents.find((entry) => entry.id === id);
    return Boolean(agent?.runtime_model_switch);
  }, [agents, conversation?.active_harness_id, isExample]);

  const [modelDisplayName, setModelDisplayName] = useState<string | undefined>();

  useEffect(() => {
    const harnessId = conversation?.active_harness_id;
    const modelId = conversation?.model_id;
    if (!harnessId || !modelId) {
      setModelDisplayName(undefined);
      return;
    }
    void loadModels(harnessId)
      .then((models) => models.find((model) => model.id === modelId)?.display_name ?? modelId)
      .then(setModelDisplayName)
      .catch(() => setModelDisplayName(modelId));
  }, [conversation?.active_harness_id, conversation?.model_id, loadModels]);

  useEffect(() => {
    closeArtifactPreview();
  }, [closeArtifactPreview, conversationId]);

  useEffect(() => {
    if (artifacts.length === 0) return;
    if (
      !autoOpenedFirstArtifact.current &&
      !sampleWorkedAt &&
      !myFileWorkedAt &&
      onboardingPhase !== "complete"
    ) {
      autoOpenedFirstArtifact.current = true;
      openArtifactPreview(conversationId, artifacts[0]!);
    }
  }, [
    artifacts,
    conversationId,
    myFileWorkedAt,
    onboardingPhase,
    openArtifactPreview,
    sampleWorkedAt,
  ]);

  useEffect(() => {
    if (dockState.tabs.length === 0 && artifactSidebarOpen) {
      closeArtifactPreview();
    }
  }, [artifactSidebarOpen, closeArtifactPreview, dockState.tabs.length]);

  useEffect(() => {
    if (!selectedArtifact || artifacts.length === 0) return;
    const stillPresent = artifacts.some(
      (artifact) => artifact.path === selectedArtifact.path && artifact.sha256 === selectedArtifact.sha256,
    );
    if (!stillPresent) {
      clearArtifactSelection();
    }
  }, [artifacts, clearArtifactSelection, selectedArtifact]);

  const onSend = useCallback(
    async (text: string) => {
      followLatestRef.current = true;
      setShowScrollToBottom(false);
      await sendMessage(text);
    },
    [sendMessage],
  );

  const scrollToBottom = useCallback((animated = true) => {
    followLatestRef.current = true;
    programmaticScrollUntilRef.current = Date.now() + 240;
    setShowScrollToBottom(false);
    transcriptRef.current?.scrollToEnd({ animated });
  }, []);

  const handleTranscriptScroll = useCallback(
    (event: NativeSyntheticEvent<NativeScrollEvent>) => {
      const { contentOffset, contentSize, layoutMeasurement } = event.nativeEvent;
      const nearBottom = isNearTranscriptBottom({
        contentHeight: contentSize.height,
        viewportHeight: layoutMeasurement.height,
        offsetY: contentOffset.y,
      });
      if (!nearBottom && Date.now() < programmaticScrollUntilRef.current) return;
      followLatestRef.current = nearBottom;
      setShowScrollToBottom(!nearBottom);
    },
    [],
  );

  const handleTranscriptContentSizeChange = useCallback(() => {
    if (!followLatestRef.current) return;
    programmaticScrollUntilRef.current = Date.now() + 160;
    transcriptRef.current?.scrollToEnd({ animated: false });
  }, []);

  const onForked = useCallback(
    (forked: ConversationDto) => {
      setForkSourceMessageId(undefined);
      router.push(`/conversation/${forked.id}`);
    },
    [router],
  );

  const openForkFromMessage = useCallback((messageId: string) => {
    setForkSourceMessageId(messageId);
    setForkOpen(true);
  }, []);

  const closeForkSheet = useCallback(() => {
    setForkOpen(false);
    setForkSourceMessageId(undefined);
  }, []);

  const attachFiles = useCallback(
    async (browserFiles: File[]) => {
      setAttaching(true);
      try {
        await attachBrowserFiles(browserFiles);
      } finally {
        setAttaching(false);
      }
    },
    [attachBrowserFiles],
  );

  const attachRoot = useCallback(async () => {
    setAttaching(true);
    try {
      await attachFilesystemRoot();
    } finally {
      setAttaching(false);
    }
  }, [attachFilesystemRoot]);

  const exportConversation = useCallback(async () => {
    const shell = shellBridge();
    if (!shell?.pickSaveFile) return;
    const defaultName = `${conversation?.title?.replace(/[^\w.-]+/g, "-") || "conversation"}.tamtri`;
    const destPath = await shell.pickSaveFile({
      title: "Export conversation bundle",
      defaultPath: defaultName,
      filters: [{ name: "tamtri bundle", extensions: ["tamtri"] }],
    });
    if (!destPath) return;
    setExporting(true);
    try {
      await client.request(method.CONVERSATION_EXPORT_BUNDLE, {
        conversation_id: conversationId,
        dest_path: destPath,
      });
    } finally {
      setExporting(false);
    }
  }, [client, conversation?.title, conversationId]);

  const handleErrorAction = useCallback(async () => {
    if (!classifiedError) return;
    if (classifiedError.kind === "conversation_busy") {
      await client.request(method.RUN_CANCEL, { conversation_id: conversationId });
      return;
    }
    if (classifiedError.kind === "harness_missing") {
      router.push("/settings/providers");
      return;
    }
    if (classifiedError.kind === "malformed_vault") {
      const folder = await client.request<string>(method.CONVERSATION_FOLDER_PATH, {
        conversation_id: conversationId,
      });
      await shellBridge()?.showItemInFolder?.(folder);
    }
  }, [classifiedError, client, conversationId, router]);

  const copyExample = useCallback(async () => {
    setCopyingExample(true);
    try {
      const forked = await client.request<ConversationDto>(method.CONVERSATION_COPY_EXAMPLE, {
        id: conversationId,
      });
      router.push(`/conversation/${forked.id}`);
    } catch {
      setForkOpen(true);
    } finally {
      setCopyingExample(false);
    }
  }, [client, conversationId, router]);

  if (loading && !conversation) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <Text style={{ color: theme.colors.foregroundMuted }}>Loading conversation…</Text>
      </View>
    );
  }

  const dockToggleDisabled = dockState.tabs.length === 0;

  return (
    <View style={{ flex: 1, flexDirection: "row", backgroundColor: theme.colors.surface0 }}>
      <View style={{ flex: 1, minWidth: 0 }}>
      <TitlebarDragRegion />
      <View
        style={{
          height: 46 + (compact ? insets.top : 0),
          paddingTop: compact ? insets.top : 0,
          paddingHorizontal: theme.spacing[3],
          borderBottomWidth: 1,
          borderBottomColor: theme.colors.border,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[3],
          backgroundColor: theme.colors.surface0,
        }}
      >
        {compact ? (
          <Pressable
            onPress={toggleSidebar}
            accessibilityRole="button"
            accessibilityLabel="Open project sidebar"
            hitSlop={8}
          >
            <PanelLeft color={theme.colors.foreground} size={18} />
          </Pressable>
        ) : null}
        <View style={{ flex: 1, minWidth: 0, flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
            <Text numberOfLines={1} style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "500", flexShrink: 1 }}>
              {conversation?.title ?? "Conversation"}
            </Text>
            {isExample ? (
              <View
                style={{
                  paddingHorizontal: theme.spacing[2],
                  paddingVertical: 2,
                  borderRadius: theme.radius.sm,
                  backgroundColor: theme.colors.surface3,
                }}
              >
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
                  {onboardingCopy.example.badge}
                </Text>
              </View>
            ) : null}
          {!compact ? (
            <View
              style={{
                maxWidth: 260,
                paddingHorizontal: 7,
                height: 22,
                borderRadius: theme.radius.md,
                backgroundColor: theme.colors.surface2,
                justifyContent: "center",
              }}
            >
              <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                {harnessDisplayName ?? conversation?.active_harness_id ?? "agent app"} · {modelDisplayName ?? conversation?.model_id ?? "model"}
              </Text>
            </View>
          ) : null}
        </View>
        {!compact ? (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
            {orchestrationEnabled ? (
              <HeaderAction
                label="Recipe"
                icon={<Workflow color={theme.colors.accentBright} size={14} />}
                onPress={() => setRecipeOpen(true)}
              />
            ) : null}
            <HeaderAction
              label={exporting ? "Saving…" : "Export"}
              icon={<Share2 color={theme.colors.accentBright} size={14} />}
              onPress={() => void exportConversation()}
            />
            <Pressable
              onPress={() => {
                if (dockToggleDisabled) return;
                if (artifactSidebarOpen) {
                  toggleArtifactSidebar();
                  return;
                }
                if (selectedArtifact) {
                  toggleArtifactSidebar();
                  return;
                }
                if (artifacts[0]) openArtifactPreview(conversationId, artifacts[0]);
                else toggleArtifactSidebar();
              }}
              hitSlop={8}
              accessibilityRole="button"
              accessibilityLabel={artifactSidebarOpen ? "Close right dock" : "Open right dock"}
              accessibilityState={{ disabled: dockToggleDisabled }}
              style={({ pressed }) => ({
                padding: theme.spacing[1],
                borderRadius: theme.radius.md,
                opacity: dockToggleDisabled ? 0.4 : 1,
                backgroundColor: pressed && !dockToggleDisabled ? theme.colors.surface2 : "transparent",
              })}
            >
              <PanelRight
                color={artifactSidebarOpen ? theme.colors.accentBright : theme.colors.foregroundMuted}
                size={18}
              />
            </Pressable>
          </View>
        ) : (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}>
            <Pressable
              onPress={() => {
                if (dockToggleDisabled) return;
                if (artifactSidebarOpen) {
                  toggleArtifactSidebar();
                  return;
                }
                if (artifacts[0]) openArtifactPreview(conversationId, artifacts[0]);
                else toggleArtifactSidebar();
              }}
              hitSlop={8}
              accessibilityRole="button"
              accessibilityLabel={artifactSidebarOpen ? "Close right dock" : "Open right dock"}
              accessibilityState={{ disabled: dockToggleDisabled }}
              style={{ opacity: dockToggleDisabled ? 0.4 : 1 }}
            >
              <PanelRight
                color={artifactSidebarOpen ? theme.colors.accentBright : theme.colors.foregroundMuted}
                size={18}
              />
            </Pressable>
            {orchestrationEnabled ? (
              <Pressable onPress={() => setRecipeOpen(true)} hitSlop={8}>
                <Workflow color={theme.colors.accentBright} size={18} />
              </Pressable>
            ) : null}
          </View>
        )}
        <Pressable
          onPress={() => {
            if (conversation?.project_id) {
              beginProjectDraft(conversation.project_id);
            }
            router.push("/");
          }}
          accessibilityRole="button"
          accessibilityLabel="New thread in this project"
        >
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs }}>
            New thread
          </Text>
        </Pressable>
      </View>

      {classifiedError ? (
        <View style={{ padding: theme.spacing[4], alignItems: "center" }}>
          <View style={{ width: "100%", maxWidth: CONVERSATION_COLUMN_WIDTH }}>
            <ErrorCard
              error={classifiedError}
              compact
              onAction={
                classifiedError.actionLabel
                  ? () => void handleErrorAction()
                  : undefined
              }
            />
          </View>
        </View>
      ) : null}

      <View style={{ flex: 1, minHeight: 0, position: "relative" }}>
        <ScrollView
          ref={transcriptRef}
          style={{ flex: 1 }}
          onScroll={handleTranscriptScroll}
          onContentSizeChange={handleTranscriptContentSizeChange}
          scrollEventThrottle={32}
          contentContainerStyle={{
            paddingHorizontal: compact ? theme.spacing[3] : theme.spacing[5],
            paddingTop: theme.spacing[6],
            paddingBottom: theme.spacing[8],
            alignItems: "center",
          }}
        >
        <View style={{ width: "100%", maxWidth: CONVERSATION_COLUMN_WIDTH }}>
          <MessageList
            uiMessages={uiMessages}
            liveMessageId={liveUiMessage?.id}
            showWorkingIndicator={isRunning && (liveUiMessage?.parts.length ?? 0) === 0}
            conversationId={conversationId}
            isCompact={compact}
            onForkMessage={openForkFromMessage}
          />
          {showExampleCard ? (
            <OnboardingCard title={onboardingCopy.example.cardTitle} body={onboardingCopy.example.cardBody} accent>
              <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2] }}>
                <Button
                  label={copyingExample ? onboardingCopy.example.usingLabel : onboardingCopy.example.useLabel}
                  compact
                  disabled={copyingExample}
                  onPress={() => void copyExample()}
                />
                <Button label={onboardingCopy.example.dismissLabel} variant="ghost" compact onPress={dismissExample} />
              </View>
            </OnboardingCard>
          ) : null}
          {isExample && readyCount === 0 ? (
            <OnboardingCard
              title="Run this on your own files"
              body="To run this on your own files, tamtri needs an agent app installed on your Mac."
              accent
            >
              <Button label="Set up an agent app" compact onPress={() => router.push("/settings/providers")} />
            </OnboardingCard>
          ) : null}
          {showDropYoursCard ? (
            <OnboardingCard
              title={onboardingCopy.payoff.dropYoursTitle}
              body={onboardingCopy.payoff.dropYoursBody}
              accent
            >
              <Button
                label={onboardingCopy.payoff.dismissLabel}
                variant="ghost"
                compact
                onPress={() => {
                  markMyFileWorked();
                  completeOnboarding();
                }}
              />
            </OnboardingCard>
          ) : null}
        </View>
        </ScrollView>
        {showScrollToBottom ? (
          <Pressable
            onPress={() => scrollToBottom()}
            accessibilityRole="button"
            accessibilityLabel="Scroll to latest message"
            style={({ pressed }) => ({
              position: "absolute",
              bottom: theme.spacing[2],
              alignSelf: "center",
              width: 34,
              height: 34,
              borderRadius: 17,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: theme.colors.surface2,
              borderWidth: 1,
              borderColor: theme.colors.borderAccent,
              opacity: pressed ? 0.78 : 1,
              ...(typeof document !== "undefined"
                ? ({ boxShadow: "0 4px 14px rgba(0,0,0,0.16)" } as Record<string, unknown>)
                : {}),
            })}
          >
            <ArrowDown color={theme.colors.foreground} size={16} />
          </Pressable>
        ) : null}
      </View>

      <View
        style={{
          paddingHorizontal: compact ? theme.spacing[3] : theme.spacing[5],
          paddingTop: theme.spacing[1],
          paddingBottom: theme.spacing[4],
          alignItems: "center",
          backgroundColor: theme.colors.surface0,
        }}
      >
        <View style={{ width: "100%", maxWidth: CONVERSATION_COLUMN_WIDTH }}>
          {pendingPermission ? (
            <View style={{ width: "92%", alignSelf: "center", marginBottom: -1, zIndex: 1 }}>
            <PermissionCard
              permission={pendingPermission}
              responding={respondingPermission}
              onRespond={(optionId) => void respondPermission(optionId)}
            />
          </View>
          ) : null}

          {pendingElicitation ? (
            <View style={{ width: "92%", alignSelf: "center", marginBottom: -1, zIndex: 1 }}>
            <ElicitationCard
              elicitation={pendingElicitation}
              responding={respondingElicitation}
              onRespond={(action, dataJson) => void respondElicitation(action, dataJson)}
            />
          </View>
          ) : null}

          <Composer
            layout="inline"
            onSend={onSend}
            onStop={isRunning ? () => void cancelRun() : undefined}
            sending={sending}
            attaching={attaching}
            attachedFiles={attachedNames}
            onPickFile={canPickFile ? () => void pickAndAttach() : undefined}
            onAttachRoot={canAttachRoot ? () => void attachRoot() : undefined}
            canAttachRoot={canAttachRoot}
            onDropFiles={attachFiles}
            disabled={blockedByConsent || isRunning || isExample}
            controlsDisabled={blockedByConsent || isRunning || isExample}
            harnessId={conversation?.active_harness_id}
            harnessDisplayName={harnessDisplayName}
            modelId={conversation?.model_id}
            modelDisplayName={modelDisplayName}
            activeMode={activeMode}
            runtimeModelSwitch={runtimeModelSwitch}
            onForkRequest={() => setForkOpen(true)}
            onModelSelect={setModel}
            placeholder={
              pendingPermission
                ? "Respond to the permission request above…"
                : pendingElicitation
                  ? "Respond to the input request above…"
                  : undefined
            }
          />
        </View>
      </View>

      <ForkConversationSheet
        visible={forkOpen}
        conversationId={conversationId}
        sourceMessageId={forkSourceMessageId}
        onClose={closeForkSheet}
        onForked={onForked}
      />
      <RunRecipeSheet
        visible={recipeOpen}
        conversationId={conversationId}
        onClose={() => setRecipeOpen(false)}
      />
      </View>

      <RightDock conversationId={conversationId} artifacts={artifacts} state={dockState} />
    </View>
  );
}
