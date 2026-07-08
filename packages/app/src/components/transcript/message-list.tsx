import { useEffect, useState } from "react";
import { Text, View } from "react-native";

import { normalizeBlock, type ContentBlock, type TranscriptMessage } from "@/lib/transcript";
import { theme } from "@/styles/theme";

function BlockView({ block }: { block: ContentBlock }) {
  switch (block.type) {
    case "text":
      return (
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, lineHeight: 24 }}>
          {block.text}
        </Text>
      );
    case "thinking":
      return (
        <View style={{ backgroundColor: theme.colors.surface2, borderRadius: theme.radius.lg, padding: theme.spacing[3], borderWidth: 1, borderColor: theme.colors.border }}>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600", marginBottom: 6 }}>
            Thinking
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>{block.text}</Text>
        </View>
      );
    case "tool_call":
      return (
        <View style={{ backgroundColor: theme.colors.surface2, borderRadius: theme.radius.lg, padding: theme.spacing[3], borderWidth: 1, borderColor: theme.colors.borderAccent }}>
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "700" }}>Tool</Text>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "600", marginTop: 4 }}>{block.name}</Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 6 }} numberOfLines={4}>
            {JSON.stringify(block.input)}
          </Text>
        </View>
      );
    case "tool_result":
      return (
        <View style={{ backgroundColor: theme.colors.surface1, borderRadius: theme.radius.lg, padding: theme.spacing[3], borderLeftWidth: 3, borderLeftColor: theme.colors.accent }}>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "700" }}>Result</Text>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, marginTop: 4 }} numberOfLines={8}>
            {typeof block.output === "string" ? block.output : JSON.stringify(block.output, null, 2)}
          </Text>
        </View>
      );
    case "artifact":
      return (
        <View style={{ backgroundColor: theme.colors.surface2, borderRadius: theme.radius.lg, padding: theme.spacing[4], borderWidth: 1, borderColor: theme.colors.border }}>
          <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{block.path.split("/").pop()}</Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
            {block.mime_type} · {block.size} bytes
          </Text>
        </View>
      );
    case "elicitation_request":
      return (
        <View style={{ backgroundColor: theme.colors.surface2, borderRadius: theme.radius.lg, padding: theme.spacing[4], borderWidth: 1, borderColor: theme.colors.accent }}>
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "700" }}>Input needed</Text>
          <Text style={{ color: theme.colors.foreground, marginTop: 8 }}>{block.message}</Text>
        </View>
      );
    default:
      return null;
  }
}

function MessageBubble({ message, streaming }: { message: TranscriptMessage; streaming?: boolean }) {
  const isUser = message.role === "user";
  const blocks = message.content.map((raw) =>
    typeof raw === "object" && raw && "type" in raw
      ? normalizeBlock(raw as Record<string, unknown>)
      : ({ type: "unknown", raw } as ContentBlock),
  );

  return (
    <View style={{ alignItems: isUser ? "flex-end" : "flex-start", marginBottom: theme.spacing[4] }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginBottom: theme.spacing[2], textTransform: "capitalize" }}>
        {message.role}
      </Text>
      <View
        style={{
          maxWidth: "100%",
          width: isUser ? "auto" : "100%",
          opacity: streaming ? 0.92 : 1,
          backgroundColor: isUser ? theme.colors.surface3 : "transparent",
          borderRadius: theme.radius.xl,
          padding: isUser ? theme.spacing[3] : 0,
          gap: theme.spacing[3],
        }}
      >
        {blocks.map((block, index) => (
          <BlockView key={`${message.id}-${index}`} block={block} />
        ))}
      </View>
    </View>
  );
}

export function MessageList({
  messages,
  liveMessageId,
  showWorkingIndicator,
}: {
  messages: TranscriptMessage[];
  liveMessageId?: string;
  showWorkingIndicator?: boolean;
}) {
  if (messages.length === 0 && !showWorkingIndicator) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", padding: theme.spacing[6] }}>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.base, textAlign: "center" }}>
          Send a message to start. Attach files from your working folder in a later milestone.
        </Text>
      </View>
    );
  }

  return (
    <View style={{ gap: theme.spacing[2] }}>
      {messages.map((message) => (
        <MessageBubble
          key={message.id}
          message={message}
          streaming={message.id === liveMessageId}
        />
      ))}
      {showWorkingIndicator ? <ThinkingIndicator /> : null}
    </View>
  );
}

export function ThinkingIndicator() {
  const [dots, setDots] = useState(".");
  useEffect(() => {
    const id = setInterval(() => setDots((d) => (d.length >= 3 ? "." : `${d}.`)), 400);
    return () => clearInterval(id);
  }, []);
  return (
    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, paddingVertical: theme.spacing[3] }}>
      Agent is working{dots}
    </Text>
  );
}
