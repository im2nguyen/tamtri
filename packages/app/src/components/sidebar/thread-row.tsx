import { ArrowRightLeft, MessageSquare } from "lucide-react-native";
import { useState } from "react";
import { Platform, Pressable, Text, View } from "react-native";
import type { ConversationSummaryDto } from "@tamtri/protocol";

import { useSidebarRowStyles } from "@/components/sidebar/sidebar-row-styles";
import { CompactIconButton } from "@/components/ui/surface-controls";
import { useTheme } from "@/styles/use-theme";

interface ThreadRowProps {
  conversation: ConversationSummaryDto;
  selected: boolean;
  onPress: () => void;
  onMove?: () => void;
}

function relativeTime(value: string): string {
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return "";
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return "now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

export function ThreadRow({ conversation, selected, onPress, onMove }: ThreadRowProps) {
  const theme = useTheme();
  const styles = useSidebarRowStyles();
  const [hovered, setHovered] = useState(false);
  const showMove = Boolean(onMove) && (hovered || Platform.OS !== "web");
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected }}
      onHoverIn={() => setHovered(true)}
      onHoverOut={() => setHovered(false)}
      onPress={onPress}
      style={({ pressed }) => [
        styles.row,
        {
          minHeight: theme.density.rowHeight,
          marginLeft: theme.spacing[6],
          paddingRight: theme.spacing[2],
        },
        selected ? styles.active : null,
        pressed ? styles.pressed : null,
      ]}
    >
      <MessageSquare
        color={selected ? theme.colors.foreground : theme.colors.foregroundMuted}
        size={13}
        strokeWidth={1.7}
      />
      <Text numberOfLines={1} style={[styles.label, { flex: 1 }]}>
        {conversation.title || "Untitled"}
      </Text>
      {showMove ? (
        <CompactIconButton
          icon={ArrowRightLeft}
          label={`Move ${conversation.title || "thread"}`}
          size={12}
          onPress={(event) => {
            event.stopPropagation();
            onMove?.();
          }}
        />
      ) : null}
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 10 }}>
        {relativeTime(conversation.updated_at)}
      </Text>
    </Pressable>
  );
}
