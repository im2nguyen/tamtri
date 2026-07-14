import {
  Brain,
  CheckCircle2,
  CircleDashed,
  AppWindow,
  ListChecks,
  Wrench,
  XCircle,
} from "lucide-react-native";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import type { DynamicToolUIPart, TamtriDataPart, TamtriUIMessage, TamtriUIMessagePart } from "@tamtri/protocol";
import { Platform, Text, View } from "react-native";

import { ArtifactCard } from "@/components/transcript/artifact-card";
import {
  AssistantMessageFooter,
  computeTurnDurationMs,
  UserMessageFooter,
} from "@/components/transcript/message-actions";
import { Disclosure } from "@/components/ui/disclosure";
import { summarizeToolActivity } from "@/lib/conversation-surface";
import { messageCopyText } from "@/lib/message-text";
import type { ContentBlock } from "@/lib/transcript";
import { parseIsoTimestamp } from "@/lib/time";
import { useTheme } from "@/styles/use-theme";

function RawReceiptContent({ value }: { value: unknown }) {
  const theme = useTheme();
  const text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
  if (!text) return null;
  return (
    <View
      style={{
        marginTop: theme.spacing[1],
        marginLeft: 22,
        padding: theme.spacing[2],
        borderRadius: theme.radius.md,
        backgroundColor: theme.colors.surface1,
        borderWidth: 1,
        borderColor: theme.colors.border,
      }}
    >
      <Text
        selectable
        style={{
          color: theme.colors.foregroundMuted,
          fontFamily: theme.fontFamily.mono,
          fontSize: theme.fontSize.xs,
          lineHeight: 18,
        }}
      >
        {text}
      </Text>
    </View>
  );
}

function ReceiptRow({
  label,
  meta,
  icon,
  children,
  streaming,
}: {
  label: string;
  meta?: string;
  icon: ReactNode;
  children: ReactNode;
  streaming?: boolean;
}) {
  const theme = useTheme();
  return (
    <View
      accessible
      accessibilityLabel={[label, meta].filter(Boolean).join(", ")}
      style={{
        borderTopWidth: 1,
        borderBottomWidth: 1,
        borderColor: theme.colors.border,
        paddingVertical: 3,
        opacity: streaming ? 0.9 : 1,
      }}
    >
      <Disclosure
        accessibilityLabel={`${label}. Show details`}
        title={
          <View style={{ flex: 1, minWidth: 0, flexDirection: "row", alignItems: "center", gap: 7 }}>
            {icon}
            <Text
              numberOfLines={1}
              style={{ flex: 1, color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}
            >
              {label}
            </Text>
            {meta ? (
              <Text style={{ color: theme.colors.foregroundMuted, opacity: 0.72, fontSize: theme.fontSize.xs }}>
                {meta}
              </Text>
            ) : null}
          </View>
        }
      >
        {children}
      </Disclosure>
    </View>
  );
}

function ThinkingReceipt({ text, streaming }: { text: string; streaming?: boolean }) {
  const theme = useTheme();
  return (
    <ReceiptRow
      label={streaming ? "Thinking…" : "Reasoning"}
      meta={streaming ? "live" : undefined}
      streaming={streaming}
      icon={<Brain color={theme.colors.foregroundMuted} size={14} />}
    >
      <RawReceiptContent value={text} />
    </ReceiptRow>
  );
}

function dataPartToArtifactBlock(part: Extract<TamtriDataPart, { type: "data-tamtri-artifact" }>): ContentBlock {
  return {
    type: "artifact",
    path: part.data.path,
    mime_type: part.data.mime_type,
    size: part.data.size,
    sha256: part.data.sha256,
    inline: part.data.inline,
    integrity_failed: part.data.integrity_failed,
  };
}

function BlockView({ block, conversationId }: { block: ContentBlock; conversationId?: string }) {
  const theme = useTheme();
  switch (block.type) {
    case "text":
      return (
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, lineHeight: 24 }}>
          {block.text}
        </Text>
      );
    case "thinking":
      return <ThinkingReceipt text={block.text} />;
    case "tool_call":
      return <LegacyToolReceipt name={block.name} input={block.input} />;
    case "tool_result":
      return (
        <ReceiptRow
          label="Tool result"
          meta="complete"
          icon={<CheckCircle2 color={theme.colors.foregroundMuted} size={14} />}
        >
          <RawReceiptContent value={block.output} />
        </ReceiptRow>
      );
    case "artifact":
      return conversationId ? (
        <ArtifactCard conversationId={conversationId} artifact={block} />
      ) : (
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

function LegacyToolReceipt({ name, input }: { name: string; input: unknown }) {
  const theme = useTheme();
  const summary = summarizeToolActivity({ toolName: name, toolInput: input, state: "input-available" });
  return (
    <ReceiptRow
      label={summary.label}
      meta="running"
      icon={<Wrench color={theme.colors.foregroundMuted} size={14} />}
    >
      <RawReceiptContent value={input} />
    </ReceiptRow>
  );
}

function ToolPartView({ part }: { part: DynamicToolUIPart }) {
  const theme = useTheme();
  const toolName = part.type.startsWith("tool-") ? part.type.slice(5) : "tool";
  const showResult =
    part.state === "output-available" ||
    part.state === "output-error" ||
    part.output !== undefined ||
    part.errorText !== undefined;

  const summary = summarizeToolActivity({
    toolName,
    toolInput: part.input,
    state: part.state,
    errorText: part.errorText,
  });
  const statusIcon =
    summary.status === "failed" ? (
      <XCircle color={theme.colors.destructive} size={14} />
    ) : summary.status === "running" ? (
      <CircleDashed color={theme.colors.accentBright} size={14} />
    ) : (
      <CheckCircle2 color={theme.colors.foregroundMuted} size={14} />
    );
  return (
    <ReceiptRow label={summary.label} meta={summary.status} icon={statusIcon} streaming={summary.status === "running"}>
      {part.input !== undefined ? <RawReceiptContent value={part.input} /> : null}
      {showResult ? <RawReceiptContent value={part.errorText ?? part.output} /> : null}
    </ReceiptRow>
  );
}

function PartView({
  part,
  conversationId,
  partKey,
}: {
  part: TamtriUIMessagePart;
  conversationId?: string;
  partKey: string;
}) {
  const theme = useTheme();
  switch (part.type) {
    case "text":
      return (
        <Text
          key={partKey}
          style={{
            color: theme.colors.foreground,
            fontSize: theme.fontSize.base,
            lineHeight: 24,
            opacity: part.state === "streaming" ? 0.95 : 1,
          }}
        >
          {part.text}
        </Text>
      );
    case "reasoning":
      return <ThinkingReceipt key={partKey} text={part.text} streaming={part.state === "streaming"} />;
    default:
      if (part.type.startsWith("tool-")) {
        return <ToolPartView key={partKey} part={part as DynamicToolUIPart} />;
      }
      if (part.type === "data-tamtri-artifact") {
        return (
          <BlockView
            key={partKey}
            block={dataPartToArtifactBlock(part as Extract<TamtriDataPart, { type: "data-tamtri-artifact" }>)}
            conversationId={conversationId}
          />
        );
      }
      if (part.type === "data-tamtri-elicitation") {
        const data = (part as Extract<TamtriDataPart, { type: "data-tamtri-elicitation" }>).data;
        if (data.phase === "request" && data.message) {
          return (
            <BlockView
              key={partKey}
              block={{
                type: "elicitation_request",
                request_id: data.request_id,
                message: data.message,
                mode: data.mode ?? "form",
              }}
            />
          );
        }
        return null;
      }
      if (part.type === "data-tamtri-task") {
        const data = (part as Extract<TamtriDataPart, { type: "data-tamtri-task" }>).data;
        return (
          <ReceiptRow
            key={partKey}
            label={data.title ?? data.task_id}
            meta={data.status}
            icon={<ListChecks color={theme.colors.accentBright} size={14} />}
          >
            <RawReceiptContent value={data.result_summary ?? data} />
          </ReceiptRow>
        );
      }
      if (part.type === "data-tamtri-app-resource") {
        const data = (part as Extract<TamtriDataPart, { type: "data-tamtri-app-resource" }>).data;
        return (
          <ReceiptRow
            key={partKey}
            label={data.uri}
            meta={data.server_id ?? "MCP App"}
            icon={<AppWindow color={theme.colors.accentBright} size={14} />}
          >
            <RawReceiptContent value={data.state} />
          </ReceiptRow>
        );
      }
      return null;
  }
}

function UiMessageBubble({
  message,
  streaming,
  conversationId,
  messageIndex,
  allMessages,
  isCompact,
  onForkMessage,
}: {
  message: TamtriUIMessage;
  streaming?: boolean;
  conversationId?: string;
  messageIndex: number;
  allMessages: TamtriUIMessage[];
  isCompact?: boolean;
  onForkMessage?: (messageId: string) => void;
}) {
  const theme = useTheme();
  const isUser = message.role === "user";
  const isAssistant = message.role === "assistant";
  const [isHovered, setIsHovered] = useState(false);
  const createdAt = parseIsoTimestamp(message.metadata?.created_at);
  const copyText = useMemo(() => messageCopyText(message), [message]);
  const hasCopyText = copyText.length > 0;
  const getCopyContent = useCallback(() => copyText, [copyText]);
  const durationMs = useMemo(
    () => (isAssistant && !streaming ? computeTurnDurationMs(messageIndex, allMessages) : undefined),
    [allMessages, isAssistant, messageIndex, streaming],
  );
  const showUserFooter = isUser && hasCopyText;
  const showAssistantFooter = isAssistant && !streaming && (hasCopyText || Boolean(onForkMessage));
  const showFooter =
    (showUserFooter || showAssistantFooter) && (isCompact || Platform.OS !== "web" || isHovered);

  const handlePointerEnter = useCallback(() => setIsHovered(true), []);
  const handlePointerLeave = useCallback(() => setIsHovered(false), []);
  const handleFork = useCallback(() => onForkMessage?.(message.id), [message.id, onForkMessage]);

  return (
    <View style={{ alignItems: isUser ? "flex-end" : "flex-start", marginBottom: theme.spacing[5] }}>
      <View
        onPointerEnter={handlePointerEnter}
        onPointerLeave={handlePointerLeave}
        style={{ maxWidth: isUser ? "82%" : "100%", width: isUser ? "auto" : "100%" }}
      >
        <View
          style={{
            maxWidth: "100%",
            width: isUser ? "auto" : "100%",
            opacity: streaming ? 0.92 : 1,
            backgroundColor: isUser ? theme.colors.surface3 : "transparent",
            borderRadius: 16,
            paddingHorizontal: isUser ? theme.spacing[3] : 0,
            paddingVertical: isUser ? theme.spacing[2] : 0,
            gap: theme.spacing[2],
          }}
        >
          {message.parts.map((part, index) => (
            <PartView
              key={`${message.id}-${part.type}-${index}`}
              partKey={`${message.id}-${part.type}-${index}`}
              part={part}
              conversationId={conversationId}
            />
          ))}
        </View>
        {showUserFooter ? (
          <UserMessageFooter getContent={getCopyContent} createdAt={createdAt} visible={showFooter} />
        ) : null}
        {showAssistantFooter ? (
          <AssistantMessageFooter
            getContent={getCopyContent}
            createdAt={createdAt}
            durationMs={durationMs}
            visible={showFooter}
            showCopy={hasCopyText}
            onFork={onForkMessage ? handleFork : undefined}
          />
        ) : null}
      </View>
    </View>
  );
}

export function MessageList({
  uiMessages,
  liveMessageId,
  showWorkingIndicator,
  conversationId,
  isCompact,
  onForkMessage,
}: {
  uiMessages: TamtriUIMessage[];
  liveMessageId?: string;
  showWorkingIndicator?: boolean;
  conversationId?: string;
  isCompact?: boolean;
  onForkMessage?: (messageId: string) => void;
}) {
  const theme = useTheme();
  if (uiMessages.length === 0 && !showWorkingIndicator) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", padding: theme.spacing[6] }}>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.base, textAlign: "center" }}>
          Send a message to start. Drop a CSV or data file onto the composer to seed the workdir.
        </Text>
      </View>
    );
  }

  return (
    <View>
      {uiMessages.map((message, index) => (
        <UiMessageBubble
          key={message.id}
          message={message}
          messageIndex={index}
          allMessages={uiMessages}
          streaming={message.id === liveMessageId}
          conversationId={conversationId}
          isCompact={isCompact}
          onForkMessage={onForkMessage}
        />
      ))}
      {showWorkingIndicator ? <ThinkingIndicator /> : null}
    </View>
  );
}

export function ThinkingIndicator() {
  const theme = useTheme();
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
