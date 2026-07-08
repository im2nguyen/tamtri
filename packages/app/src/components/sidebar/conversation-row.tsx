import { Pressable, Text, View } from "react-native";
import type { ConversationSummaryDto } from "@tamtri/protocol";

import { theme } from "@/styles/theme";

interface ConversationRowProps {
  conversation: ConversationSummaryDto;
  selected?: boolean;
  onPress: () => void;
}

function formatRelativeTime(epochSeconds: number): string {
  const delta = Math.max(0, Math.floor(Date.now() / 1000 - epochSeconds));
  if (delta < 60) return "now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h`;
  return `${Math.floor(delta / 86400)}d`;
}

export function ConversationRow({ conversation, selected, onPress }: ConversationRowProps) {
  return (
    <Pressable
      onPress={onPress}
      style={({ pressed, hovered }) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: theme.spacing[3],
        paddingHorizontal: theme.spacing[3],
        paddingVertical: theme.spacing[3],
        borderRadius: theme.radius.lg,
        backgroundColor: selected
          ? theme.colors.surface2
          : pressed || hovered
            ? theme.colors.surfaceSidebarHover
            : "transparent",
      })}
    >
      <View
        style={{
          width: 8,
          height: 8,
          borderRadius: theme.radius.full,
          backgroundColor: conversation.active_harness_id ? theme.colors.accentBright : theme.colors.surface3,
        }}
      />
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text
          numberOfLines={1}
          style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "500" }}
        >
          {conversation.title || "Untitled"}
        </Text>
        <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 2 }}>
          {conversation.active_harness_id ?? "no harness"} · {formatRelativeTime(Number(conversation.updated_at))}
        </Text>
      </View>
    </Pressable>
  );
}
