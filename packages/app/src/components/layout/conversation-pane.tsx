import { useRouter } from "expo-router";
import { GitBranch, PanelLeft, Workflow } from "lucide-react-native";
import { useCallback, useState, type ReactNode } from "react";
import { Pressable, ScrollView, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import type { ConversationDto } from "@tamtri/protocol";

import { Composer } from "@/components/composer/composer";
import { ElicitationCard } from "@/components/consent/elicitation-card";
import { PermissionCard } from "@/components/consent/permission-card";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { RunRecipeSheet } from "@/components/orchestration/run-recipe-sheet";
import { ForkConversationSheet } from "@/components/sidebar/fork-conversation-sheet";
import { MessageList } from "@/components/transcript/message-list";
import { isCompact, MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useConversation } from "@/hooks/use-conversation";
import { useDaemon } from "@/runtime/daemon-provider";
import { useUiStore } from "@/stores/ui-store";
import { theme } from "@/styles/theme";

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
  return (
    <Pressable
      onPress={onPress}
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
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const { serverInfo } = useDaemon();
  const orchestrationEnabled = Boolean(serverInfo?.features.orchestration);
  const [forkOpen, setForkOpen] = useState(false);
  const [recipeOpen, setRecipeOpen] = useState(false);
  const {
    conversation,
    messages,
    liveMessage,
    loading,
    sending,
    isRunning,
    pendingPermission,
    respondingPermission,
    pendingElicitation,
    respondingElicitation,
    error,
    sendMessage,
    respondPermission,
    respondElicitation,
  } = useConversation(conversationId);

  const blockedByConsent = Boolean(pendingPermission || pendingElicitation);

  const onSend = useCallback(
    async (text: string) => {
      await sendMessage(text);
    },
    [sendMessage],
  );

  const onForked = useCallback(
    (forked: ConversationDto) => {
      router.push(`/conversation/${forked.id}`);
    },
    [router],
  );

  if (loading && !conversation) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <Text style={{ color: theme.colors.foregroundMuted }}>Loading conversation…</Text>
      </View>
    );
  }

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <TitlebarDragRegion />
      <View
        style={{
          height: theme.layout.headerHeight + (compact ? insets.top : 0),
          paddingTop: compact ? insets.top : 0,
          paddingHorizontal: theme.spacing[4],
          borderBottomWidth: 1,
          borderBottomColor: theme.colors.border,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[3],
          backgroundColor: theme.colors.surfaceWorkspace,
        }}
      >
        {compact ? (
          <Pressable onPress={toggleSidebar} hitSlop={8}>
            <PanelLeft color={theme.colors.foreground} size={18} />
          </Pressable>
        ) : null}
        <View style={{ flex: 1, minWidth: 0 }}>
          <Text numberOfLines={1} style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "600" }}>
            {conversation?.title ?? "Conversation"}
          </Text>
          <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }}>
            {conversation?.active_harness_id ?? "harness"} · {conversation?.model_id ?? "model"}
          </Text>
        </View>
        {!compact ? (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
            <HeaderAction
              label="Fork"
              icon={<GitBranch color={theme.colors.accentBright} size={14} />}
              onPress={() => setForkOpen(true)}
            />
            {orchestrationEnabled ? (
              <HeaderAction
                label="Recipe"
                icon={<Workflow color={theme.colors.accentBright} size={14} />}
                onPress={() => setRecipeOpen(true)}
              />
            ) : null}
          </View>
        ) : (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}>
            <Pressable onPress={() => setForkOpen(true)} hitSlop={8}>
              <GitBranch color={theme.colors.accentBright} size={18} />
            </Pressable>
            {orchestrationEnabled ? (
              <Pressable onPress={() => setRecipeOpen(true)} hitSlop={8}>
                <Workflow color={theme.colors.accentBright} size={18} />
              </Pressable>
            ) : null}
          </View>
        )}
        <Pressable onPress={() => router.push("/")}>
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>Home</Text>
        </Pressable>
      </View>

      {error ? (
        <View style={{ padding: theme.spacing[3], backgroundColor: theme.colors.destructive }}>
          <Text style={{ color: theme.colors.destructiveForeground }}>{error}</Text>
        </View>
      ) : null}

      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{
          paddingHorizontal: theme.spacing[4],
          paddingVertical: theme.spacing[6],
          alignItems: "center",
        }}
      >
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH }}>
          <MessageList
            messages={messages}
            liveMessageId={liveMessage?.id}
            showWorkingIndicator={isRunning && !liveMessage}
          />
        </View>
      </ScrollView>

      {pendingPermission ? (
        <View
          style={{
            paddingHorizontal: theme.spacing[4],
            paddingBottom: theme.spacing[3],
            alignItems: "center",
            backgroundColor: theme.colors.surfaceWorkspace,
          }}
        >
          <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH }}>
            <PermissionCard
              permission={pendingPermission}
              responding={respondingPermission}
              onRespond={(optionId) => void respondPermission(optionId)}
            />
          </View>
        </View>
      ) : null}

      {pendingElicitation ? (
        <View
          style={{
            paddingHorizontal: theme.spacing[4],
            paddingBottom: theme.spacing[3],
            alignItems: "center",
            backgroundColor: theme.colors.surfaceWorkspace,
          }}
        >
          <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH }}>
            <ElicitationCard
              elicitation={pendingElicitation}
              responding={respondingElicitation}
              onRespond={(action, dataJson) => void respondElicitation(action, dataJson)}
            />
          </View>
        </View>
      ) : null}

      <Composer
        onSend={onSend}
        sending={sending}
        disabled={blockedByConsent || isRunning}
        placeholder={
          pendingPermission
            ? "Respond to the permission request above…"
            : pendingElicitation
              ? "Respond to the input request above…"
              : undefined
        }
      />

      <ForkConversationSheet
        visible={forkOpen}
        conversationId={conversationId}
        onClose={() => setForkOpen(false)}
        onForked={onForked}
      />
      <RunRecipeSheet
        visible={recipeOpen}
        conversationId={conversationId}
        onClose={() => setRecipeOpen(false)}
      />
    </View>
  );
}

export function HomePane() {
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);

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
      <Text style={{ color: theme.colors.foreground, fontSize: 28, fontWeight: "700", marginBottom: theme.spacing[3] }}>
        Turn data into reports
      </Text>
      <Text
        style={{
          color: theme.colors.foregroundMuted,
          fontSize: theme.fontSize.base,
          textAlign: "center",
          maxWidth: 420,
          lineHeight: 24,
        }}
      >
        Pick a conversation from the sidebar or start a new one. tamtri renders harness output inline — artifacts, tools, and gateway primitives included.
      </Text>
    </View>
  );
}
