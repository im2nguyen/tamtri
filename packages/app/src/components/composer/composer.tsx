import { ArrowUp, Mic, Square } from "lucide-react-native";
import { useMemo, useState } from "react";
import {
  ActivityIndicator,
  Platform,
  Pressable,
  TextInput,
  View,
  type NativeSyntheticEvent,
  type TextInputKeyPressEventData,
} from "react-native";

import {
  AttachmentPill,
  buildDefaultAttachmentItems,
  ComposerAttachmentMenu,
} from "@/components/composer/composer-attachment-menu";
import { ComposerControls } from "@/components/composer/composer-controls";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useTheme } from "@/styles/use-theme";

interface ComposerProps {
  onSend: (text: string) => Promise<void>;
  onStop?: () => void;
  disabled?: boolean;
  sending?: boolean;
  attaching?: boolean;
  placeholder?: string;
  attachedFiles?: string[];
  onPickFile?: () => void;
  onAttachRoot?: () => void;
  canAttachRoot?: boolean;
  onDropFiles?: (files: File[]) => Promise<void>;
  harnessId?: string;
  harnessDisplayName?: string;
  modelId?: string;
  modelDisplayName?: string;
  activeMode?: string | null;
  runtimeModelSwitch?: boolean;
  controlsDisabled?: boolean;
  onForkRequest?: () => void;
  onOpenHarnessPicker?: () => void;
  onOpenModelPicker?: () => void;
  layout?: "docked" | "inline";
}

export function Composer({
  onSend,
  onStop,
  disabled,
  sending,
  attaching,
  placeholder = "Message tamtri…",
  attachedFiles = [],
  onPickFile,
  onAttachRoot,
  canAttachRoot,
  onDropFiles,
  harnessId,
  harnessDisplayName,
  modelId,
  modelDisplayName,
  activeMode,
  runtimeModelSwitch,
  controlsDisabled,
  onForkRequest,
  onOpenHarnessPicker,
  onOpenModelPicker,
  layout = "docked",
}: ComposerProps) {
  const theme = useTheme();
  const [text, setText] = useState("");
  const [dragActive, setDragActive] = useState(false);

  const submit = async () => {
    const value = text.trim();
    if (!value || disabled || sending) return;
    setText("");
    await onSend(value);
  };

  const handleDrop = async (event: DragEvent) => {
    event.preventDefault();
    setDragActive(false);
    if (!onDropFiles || disabled || sending || attaching) return;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length > 0) await onDropFiles(files);
  };

  const webDropProps =
    Platform.OS === "web"
      ? ({
          onDragOver: (event: DragEvent) => {
            event.preventDefault();
            setDragActive(true);
          },
          onDragLeave: () => setDragActive(false),
          onDrop: (event: DragEvent) => {
            handleDrop(event);
          },
        } as Record<string, unknown>)
      : {};

  const attachmentMenuItems = useMemo(
    () =>
      buildDefaultAttachmentItems({
        onAttachFile: onPickFile,
        onAttachRoot,
        canAttachRoot,
        iconColor: theme.colors.foregroundMuted,
      }),
    [canAttachRoot, onAttachRoot, onPickFile, theme.colors.foregroundMuted],
  );

  const canSend = Boolean(text.trim()) && !disabled && !sending;

  const isNewlineKeyIntent = (shiftKey: boolean, altKey: boolean) => shiftKey || altKey;

  const trySubmitFromKeyboard = () => {
    const value = text.trim();
    if (!value || disabled || sending) return false;
    void submit();
    return true;
  };

  type KeyPressWithModifiers = TextInputKeyPressEventData & {
    shiftKey?: boolean;
    altKey?: boolean;
  };

  const handleNativeKeyPress = (event: NativeSyntheticEvent<TextInputKeyPressEventData>) => {
    if (event.nativeEvent.key !== "Enter") return;
    const { shiftKey = false, altKey = false } = event.nativeEvent as KeyPressWithModifiers;
    if (isNewlineKeyIntent(shiftKey, altKey)) return;
    if (!trySubmitFromKeyboard()) return;
    event.preventDefault();
  };

  const webInputKeyProps =
    Platform.OS === "web"
      ? ({
          onKeyDown: (event: KeyboardEvent) => {
            if (event.key !== "Enter") return;
            if (event.isComposing) return;
            if (isNewlineKeyIntent(event.shiftKey, event.altKey)) return;
            if (!trySubmitFromKeyboard()) return;
            event.preventDefault();
          },
        } as Record<string, unknown>)
      : {};

  return (
    <View
      {...webDropProps}
      style={
        layout === "inline"
          ? { width: "100%", alignItems: "center" }
          : {
              paddingHorizontal: theme.spacing[4],
              paddingTop: theme.spacing[2],
              paddingBottom: theme.spacing[4],
              alignItems: "center",
            }
      }
    >
      <View
        style={{
          width: "100%",
          maxWidth: MAX_CONTENT_WIDTH,
          backgroundColor: theme.colors.surface1,
          borderRadius: 22,
          borderWidth: 1,
          borderColor: dragActive ? theme.colors.accent : theme.colors.borderAccent,
          paddingHorizontal: theme.spacing[3],
          paddingTop: theme.spacing[3],
          paddingBottom: theme.spacing[2],
          gap: theme.spacing[2],
          ...(Platform.OS === "web"
            ? ({
                boxShadow: "0 12px 30px rgba(0,0,0,0.12), 0 2px 8px rgba(0,0,0,0.08)",
              } as Record<string, unknown>)
            : {}),
        }}
      >
        {attachedFiles.length > 0 ? (
          <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2] }}>
            {attachedFiles.map((name) => (
              <AttachmentPill key={name} name={name} />
            ))}
          </View>
        ) : null}

        <TextInput
          {...webInputKeyProps}
          value={text}
          onChangeText={setText}
          placeholder={dragActive ? "Drop files to attach…" : placeholder}
          placeholderTextColor={theme.colors.foregroundMuted}
          multiline
          editable={!disabled && !sending}
          onKeyPress={Platform.OS !== "web" ? handleNativeKeyPress : undefined}
          blurOnSubmit={false}
          style={{
            color: theme.colors.foreground,
            fontSize: theme.fontSize.base,
            minHeight: 44,
            maxHeight: 160,
            paddingVertical: theme.spacing[1],
            paddingHorizontal: theme.spacing[1],
            ...(Platform.OS === "web"
              ? ({ outlineStyle: "none", outlineWidth: 0 } as Record<string, unknown>)
              : {}),
          }}
        />

        <View
          style={{
            flexDirection: "row",
            alignItems: "flex-end",
            justifyContent: "space-between",
            gap: theme.spacing[2],
          }}
        >
          <View
            style={{
              flex: 1,
              minWidth: 0,
              flexDirection: "row",
              alignItems: "flex-end",
              gap: theme.spacing[1],
            }}
          >
            <ComposerAttachmentMenu disabled={disabled} attaching={attaching} items={attachmentMenuItems} />
            <ComposerControls
              harnessId={harnessId}
              harnessDisplayName={harnessDisplayName}
              modelId={modelId}
              modelDisplayName={modelDisplayName}
              activeMode={activeMode}
              runtimeModelSwitch={runtimeModelSwitch}
              controlsDisabled={controlsDisabled}
              onForkRequest={onForkRequest}
              onOpenHarnessPicker={onOpenHarnessPicker}
              onOpenModelPicker={onOpenModelPicker}
            />
          </View>

          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}>
            <Pressable
              disabled
              accessibilityLabel="Voice input coming soon"
              accessibilityRole="button"
              style={{
                width: 28,
                height: 28,
                borderRadius: theme.radius.full,
                alignItems: "center",
                justifyContent: "center",
                opacity: 0.35,
              }}
            >
              <Mic color={theme.colors.foregroundMuted} size={16} />
            </Pressable>
            <Pressable
              testID="composer-send-button"
              onPress={onStop ?? (() => void submit())}
              disabled={!onStop && !canSend}
              accessibilityLabel={onStop ? "Stop response" : "Send message"}
              accessibilityRole="button"
              style={({ pressed }) => ({
                width: 32,
                height: 32,
                borderRadius: theme.radius.full,
                alignItems: "center",
                justifyContent: "center",
                backgroundColor: onStop || canSend ? theme.colors.accent : theme.colors.surface3,
                opacity: pressed && (onStop || canSend) ? 0.9 : 1,
              })}
            >
              {onStop ? (
                <Square color={theme.colors.accentForeground} fill={theme.colors.accentForeground} size={12} />
              ) : sending ? (
                <ActivityIndicator color={theme.colors.accentForeground} size="small" />
              ) : (
                <ArrowUp
                  color={canSend ? theme.colors.accentForeground : theme.colors.foregroundMuted}
                  size={16}
                />
              )}
            </Pressable>
          </View>
        </View>
      </View>
    </View>
  );
}
