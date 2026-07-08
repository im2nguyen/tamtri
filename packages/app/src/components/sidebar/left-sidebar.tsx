import { useRouter, usePathname } from "expo-router";
import { Download, MessageSquarePlus, PanelLeft, Search, Settings2 } from "lucide-react-native";
import { useCallback, useState } from "react";
import {
  ActivityIndicator,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  useWindowDimensions,
  View,
} from "react-native";
import { method, type ConversationDto } from "@tamtri/protocol";

import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { ConversationRow } from "@/components/sidebar/conversation-row";
import { Button } from "@/components/ui/button";
import { isCompact, SIDEBAR_WIDTH } from "@/constants/layout";
import { useConversationList } from "@/hooks/use-conversations";
import { useDaemon } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

interface LeftSidebarProps {
  onClose?: () => void;
}

export function LeftSidebar({ onClose }: LeftSidebarProps) {
  const router = useRouter();
  const pathname = usePathname();
  const { width } = useWindowDimensions();
  const compact = isCompact(width);
  const { client } = useDaemon();
  const { conversations, loading, refresh } = useConversationList();
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState(false);

  const filtered = conversations.filter((row) =>
    row.title.toLowerCase().includes(query.trim().toLowerCase()),
  );

  const createConversation = useCallback(async () => {
    setCreating(true);
    try {
      const created = await client.request<ConversationDto>(method.CONVERSATION_CREATE, {
        title: "New conversation",
        harness_id: "claude-native",
        model_id: "default",
      });
      await refresh();
      router.push(`/conversation/${created.id}`);
      onClose?.();
    } finally {
      setCreating(false);
    }
  }, [client, onClose, refresh, router]);

  return (
    <View
      style={{
        width: compact ? "100%" : SIDEBAR_WIDTH,
        flex: compact ? 1 : undefined,
        backgroundColor: theme.colors.surfaceSidebar,
        borderRightWidth: compact ? 0 : 1,
        borderRightColor: theme.colors.border,
        paddingTop: compact ? 12 : 40,
      }}
    >
      <TitlebarDragRegion />
      <View style={{ paddingHorizontal: theme.spacing[4], paddingBottom: theme.spacing[3] }}>
        <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>tamtri</Text>
          {compact ? (
            <Pressable onPress={onClose} hitSlop={8}>
              <PanelLeft color={theme.colors.foregroundMuted} size={18} />
            </Pressable>
          ) : null}
        </View>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
          Agent workspace
        </Text>
      </View>

      <View style={{ paddingHorizontal: theme.spacing[3], gap: theme.spacing[2], marginBottom: theme.spacing[3] }}>
        <View
          style={{
            flexDirection: "row",
            alignItems: "center",
            gap: theme.spacing[2],
            backgroundColor: theme.colors.surface2,
            borderRadius: theme.radius.lg,
            paddingHorizontal: theme.spacing[3],
            borderWidth: 1,
            borderColor: theme.colors.borderAccent,
          }}
        >
          <Search color={theme.colors.foregroundMuted} size={16} />
          <TextInput
            value={query}
            onChangeText={setQuery}
            placeholder="Search conversations"
            placeholderTextColor={theme.colors.foregroundMuted}
            style={{ flex: 1, color: theme.colors.foreground, paddingVertical: theme.spacing[3], fontSize: theme.fontSize.sm }}
          />
        </View>
        <Button
          label={creating ? "Creating…" : "New conversation"}
          onPress={() => void createConversation()}
          disabled={creating}
        />
      </View>

      <ScrollView style={{ flex: 1 }} contentContainerStyle={{ paddingHorizontal: theme.spacing[2], paddingBottom: 24 }}>
        {loading ? (
          <ActivityIndicator color={theme.colors.accentBright} style={{ marginTop: 24 }} />
        ) : filtered.length === 0 ? (
          <Text style={{ color: theme.colors.foregroundMuted, textAlign: "center", marginTop: 24, fontSize: theme.fontSize.sm }}>
            {query ? "No matches" : "No conversations yet"}
          </Text>
        ) : (
          filtered.map((conversation) => (
            <ConversationRow
              key={conversation.id}
              conversation={conversation}
              selected={pathname === `/conversation/${conversation.id}`}
              onPress={() => {
                router.push(`/conversation/${conversation.id}`);
                onClose?.();
              }}
            />
          ))
        )}
      </ScrollView>

      <View style={{ borderTopWidth: 1, borderTopColor: theme.colors.border, padding: theme.spacing[3], gap: theme.spacing[2] }}>
        <Pressable
          onPress={() => {
            router.push("/sessions");
            onClose?.();
          }}
          style={({ hovered, pressed }) => ({
            flexDirection: "row",
            alignItems: "center",
            gap: theme.spacing[3],
            padding: theme.spacing[3],
            borderRadius: theme.radius.lg,
            backgroundColor: pressed || hovered ? theme.colors.surfaceSidebarHover : "transparent",
          })}
        >
          <Download color={theme.colors.foregroundMuted} size={16} />
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm }}>Import sessions</Text>
        </Pressable>
        <Pressable
          style={({ hovered, pressed }) => ({
            flexDirection: "row",
            alignItems: "center",
            gap: theme.spacing[3],
            padding: theme.spacing[3],
            borderRadius: theme.radius.lg,
            backgroundColor: pressed || hovered ? theme.colors.surfaceSidebarHover : "transparent",
          })}
        >
          <Settings2 color={theme.colors.foregroundMuted} size={16} />
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>Settings (soon)</Text>
        </Pressable>
        <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2], paddingHorizontal: theme.spacing[2] }}>
          <MessageSquarePlus color={theme.colors.accentBright} size={14} />
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
            Gateway-owned capabilities render here
          </Text>
        </View>
      </View>
    </View>
  );
}
