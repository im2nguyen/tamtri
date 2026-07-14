import { Check, Copy, GitBranch } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Platform, Pressable, Text, View, type StyleProp, type ViewStyle } from "react-native";

import { copyTextToClipboard } from "@/lib/copy-text";
import { formatDuration, formatMessageTimestamp, parseIsoTimestamp } from "@/lib/time";
import { useTheme } from "@/styles/use-theme";

export const MESSAGE_METADATA_FONT_SIZE = 13;
const TIMESTAMP_REVEAL_MS = 3000;
const isWeb = Platform.OS === "web";

interface CopyButtonProps {
  getContent: () => string;
  containerStyle?: StyleProp<ViewStyle>;
  accessibilityLabel?: string;
}

function CopyButton({ getContent, containerStyle, accessibilityLabel = "Copy message" }: CopyButtonProps) {
  const theme = useTheme();
  const [copied, setCopied] = useState(false);
  const [hovered, setHovered] = useState(false);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleCopy = useCallback(async () => {
    const content = getContent();
    if (!content) return;
    const ok = await copyTextToClipboard(content);
    if (!ok) return;
    setCopied(true);
    if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    copyTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copyTimeoutRef.current = null;
    }, 1500);
  }, [getContent]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const color = hovered ? theme.colors.foreground : theme.colors.foregroundMuted;

  return (
    <Pressable
      onPress={() => void handleCopy()}
      onHoverIn={() => setHovered(true)}
      onHoverOut={() => setHovered(false)}
      style={containerStyle}
      accessibilityRole="button"
      accessibilityLabel={copied ? "Copied" : accessibilityLabel}
      hitSlop={6}
    >
      {copied ? <Check size={14} color={color} /> : <Copy size={14} color={color} />}
    </Pressable>
  );
}

interface ForkButtonProps {
  onPress: () => void;
  containerStyle?: StyleProp<ViewStyle>;
}

function ForkButton({ onPress, containerStyle }: ForkButtonProps) {
  const theme = useTheme();
  const [hovered, setHovered] = useState(false);
  return (
    <Pressable
      onPress={onPress}
      onHoverIn={() => setHovered(true)}
      onHoverOut={() => setHovered(false)}
      style={containerStyle}
      accessibilityRole="button"
      accessibilityLabel="Fork from here"
      hitSlop={6}
    >
      <GitBranch size={14} color={hovered ? theme.colors.foreground : theme.colors.foregroundMuted} />
    </Pressable>
  );
}

interface DurationLabelProps {
  durationMs?: number;
  completedAt?: Date;
}

function DurationLabel({ durationMs, completedAt }: DurationLabelProps) {
  const theme = useTheme();
  const [hovered, setHovered] = useState(false);
  const [pressedReveal, setPressedReveal] = useState(false);
  const revealTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (revealTimerRef.current) clearTimeout(revealTimerRef.current);
    };
  }, []);

  const durationLabel = useMemo(
    () => (durationMs !== undefined ? `Worked for ${formatDuration(durationMs)}` : ""),
    [durationMs],
  );
  const timestampLabel = useMemo(
    () => (completedAt ? formatMessageTimestamp(completedAt) : ""),
    [completedAt],
  );

  const canSwap = Boolean(timestampLabel && durationLabel);
  const showTimestamp = canSwap && (isWeb ? hovered : pressedReveal);

  const handleHoverIn = useCallback(() => setHovered(true), []);
  const handleHoverOut = useCallback(() => setHovered(false), []);
  const handlePress = useCallback(() => {
    if (isWeb || !canSwap) return;
    if (revealTimerRef.current) clearTimeout(revealTimerRef.current);
    setPressedReveal((prev) => !prev);
    revealTimerRef.current = setTimeout(() => {
      setPressedReveal(false);
      revealTimerRef.current = null;
    }, TIMESTAMP_REVEAL_MS);
  }, [canSwap]);

  const label = showTimestamp ? timestampLabel : durationLabel || timestampLabel;
  if (!label) return null;

  return (
    <Pressable
      onPress={handlePress}
      onHoverIn={handleHoverIn}
      onHoverOut={handleHoverOut}
      accessibilityRole={canSwap ? "button" : undefined}
      accessibilityLabel={canSwap ? `${durationLabel}, ended ${timestampLabel}` : label}
    >
      <View style={{ position: "relative" }}>
        {canSwap ? (
          <Text
            style={{
              color: theme.colors.foregroundMuted,
              fontSize: MESSAGE_METADATA_FONT_SIZE,
              opacity: 0,
            }}
            aria-hidden
          >
            {durationLabel.length >= timestampLabel.length ? durationLabel : timestampLabel}
          </Text>
        ) : null}
        <Text
          style={{
            color: theme.colors.foregroundMuted,
            fontSize: MESSAGE_METADATA_FONT_SIZE,
            ...(canSwap ? { position: "absolute", top: 0, left: 0 } : null),
          }}
        >
          {label}
        </Text>
      </View>
    </Pressable>
  );
}

interface MessageFooterRowProps {
  visible: boolean;
  align: "start" | "end";
  children: React.ReactNode;
}

function MessageFooterRow({ visible, align, children }: MessageFooterRowProps) {
  const theme = useTheme();
  const isNative = Platform.OS !== "web";
  const opacity = visible ? 1 : isNative ? 0.55 : 0;

  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        alignSelf: align === "end" ? "flex-end" : "flex-start",
        gap: theme.spacing[2],
        marginTop: theme.spacing[2],
        minHeight: 22,
        opacity,
        pointerEvents: visible || isNative ? "auto" : "none",
      }}
    >
      {children}
    </View>
  );
}

interface UserMessageFooterProps {
  getContent: () => string;
  createdAt?: Date;
  visible: boolean;
}

export function UserMessageFooter({ getContent, createdAt, visible }: UserMessageFooterProps) {
  const theme = useTheme();
  const timestampLabel = createdAt ? formatMessageTimestamp(createdAt) : "";

  return (
    <MessageFooterRow visible={visible} align="end">
      {timestampLabel ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: MESSAGE_METADATA_FONT_SIZE }}>
          {timestampLabel}
        </Text>
      ) : null}
      <CopyButton getContent={getContent} accessibilityLabel="Copy message" />
    </MessageFooterRow>
  );
}

interface AssistantMessageFooterProps {
  getContent: () => string;
  createdAt?: Date;
  durationMs?: number;
  visible: boolean;
  showCopy?: boolean;
  onFork?: () => void;
}

export function AssistantMessageFooter({
  getContent,
  createdAt,
  durationMs,
  visible,
  showCopy = true,
  onFork,
}: AssistantMessageFooterProps) {
  return (
    <MessageFooterRow visible={visible} align="start">
      {showCopy ? <CopyButton getContent={getContent} accessibilityLabel="Copy turn" /> : null}
      {onFork ? <ForkButton onPress={onFork} /> : null}
      <DurationLabel durationMs={durationMs} completedAt={createdAt} />
    </MessageFooterRow>
  );
}

export function computeTurnDurationMs(
  messageIndex: number,
  messages: Array<{ role: string; metadata?: { created_at?: string } }>,
): number | undefined {
  const message = messages[messageIndex];
  if (!message || message.role !== "assistant") return undefined;

  const endAt = parseIsoTimestamp(message.metadata?.created_at);
  if (!endAt) return undefined;

  for (let i = messageIndex - 1; i >= 0; i -= 1) {
    const prev = messages[i];
    if (prev?.role === "user") {
      const startAt = parseIsoTimestamp(prev.metadata?.created_at);
      if (startAt) {
        const duration = endAt.getTime() - startAt.getTime();
        return duration > 0 ? duration : undefined;
      }
    }
  }
  return undefined;
}
