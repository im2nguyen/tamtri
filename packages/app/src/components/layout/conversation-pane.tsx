import { useRouter } from "expo-router";
import { PanelLeft } from "lucide-react-native";
import { useCallback } from "react";
import { Pressable, ScrollView, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { Composer } from "@/components/composer/composer";
import { PermissionCard } from "@/components/consent/permission-card";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { MessageList } from "@/components/transcript/message-list";
import { isCompact, MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useConversation } from "@/hooks/use-conversation";
import { useUiStore } from "@/stores/ui-store";
import { theme } from "@/styles/theme";

interface ConversationPaneProps {
  conversationId: string;
}

export function ConversationPane({ conversationId }: ConversationPaneProps) {
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const {
    conversation,
    messages,
    liveMessage,
    loading,
    sending,
    isRunning,
    pendingPermission,
    respondingPermission,
    error,
    sendMessage,
    respondPermission,
  } = useConversation(conversationId);

  const onSend = useCallback(
    async (text: string) => {
      await sendMessage(text);
    },
    [sendMessage],
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

      <Composer
        onSend={onSend}
        sending={sending}
        disabled={Boolean(pendingPermission) || isRunning}
        placeholder={pendingPermission ? "Respond to the permission request above…" : undefined}
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
