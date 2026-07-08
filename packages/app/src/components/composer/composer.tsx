import { ArrowUp, Paperclip } from "lucide-react-native";
import { useState } from "react";
import { ActivityIndicator, Platform, Pressable, Text, TextInput, View } from "react-native";

import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { theme } from "@/styles/theme";

interface ComposerProps {
  onSend: (text: string) => Promise<void>;
  disabled?: boolean;
  sending?: boolean;
  attaching?: boolean;
  placeholder?: string;
  attachedFiles?: string[];
  onPickFile?: () => void;
  onDropFiles?: (files: File[]) => Promise<void>;
}

export function Composer({
  onSend,
  disabled,
  sending,
  attaching,
  placeholder = "Message tamtri…",
  attachedFiles = [],
  onPickFile,
  onDropFiles,
}: ComposerProps) {
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
          onDrop: (event: DragEvent) => void handleDrop(event),
        } as Record<string, unknown>)
      : {};

  return (
    <View
      {...webDropProps}
      style={{
        borderTopWidth: 1,
        borderTopColor: theme.colors.border,
        backgroundColor: dragActive ? theme.colors.surface3 : theme.colors.surfaceWorkspace,
        paddingHorizontal: theme.spacing[4],
        paddingTop: theme.spacing[3],
        paddingBottom: theme.spacing[4],
        alignItems: "center",
      }}
    >
      <View
        style={{
          width: "100%",
          maxWidth: MAX_CONTENT_WIDTH,
          backgroundColor: theme.colors.surface2,
          borderRadius: theme.radius.xl,
          borderWidth: 1,
          borderColor: dragActive ? theme.colors.accent : theme.colors.borderAccent,
          paddingHorizontal: theme.spacing[3],
          paddingVertical: theme.spacing[2],
        }}
      >
        {attachedFiles.length > 0 ? (
          <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2], marginBottom: theme.spacing[2] }}>
            {attachedFiles.map((name) => (
              <View
                key={name}
                style={{
                  paddingHorizontal: theme.spacing[2],
                  paddingVertical: 4,
                  borderRadius: theme.radius.full,
                  backgroundColor: theme.colors.surface3,
                }}
              >
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>{name}</Text>
              </View>
            ))}
          </View>
        ) : null}

        <TextInput
          value={text}
          onChangeText={setText}
          placeholder={dragActive ? "Drop files to attach…" : placeholder}
          placeholderTextColor={theme.colors.foregroundMuted}
          multiline
          editable={!disabled && !sending}
          onSubmitEditing={() => void submit()}
          blurOnSubmit={false}
          style={{
            color: theme.colors.foreground,
            fontSize: theme.fontSize.base,
            minHeight: 44,
            maxHeight: 160,
            paddingVertical: theme.spacing[2],
          }}
        />
        <View style={{ flexDirection: "row", justifyContent: "space-between", alignItems: "center", marginTop: theme.spacing[2] }}>
          <View style={{ flexDirection: "row", gap: theme.spacing[2], alignItems: "center" }}>
            <View
              style={{
                paddingHorizontal: theme.spacing[2],
                paddingVertical: 4,
                borderRadius: theme.radius.full,
                backgroundColor: theme.colors.surface3,
              }}
            >
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>VaultLocal</Text>
            </View>
            {onPickFile ? (
              <Pressable onPress={onPickFile} hitSlop={8} disabled={disabled || attaching}>
                {attaching ? (
                  <ActivityIndicator color={theme.colors.foregroundMuted} size="small" />
                ) : (
                  <Paperclip color={theme.colors.foregroundMuted} size={16} />
                )}
              </Pressable>
            ) : null}
          </View>
          <Pressable
            onPress={() => void submit()}
            disabled={disabled || sending || !text.trim()}
            style={({ pressed }) => ({
              width: 32,
              height: 32,
              borderRadius: theme.radius.full,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: text.trim() && !disabled ? theme.colors.accent : theme.colors.surface3,
              opacity: pressed ? 0.85 : 1,
            })}
          >
            {sending ? (
              <ActivityIndicator color={theme.colors.accentForeground} size="small" />
            ) : (
              <ArrowUp color={text.trim() && !disabled ? theme.colors.accentForeground : theme.colors.foregroundMuted} size={16} />
            )}
          </Pressable>
        </View>
      </View>
    </View>
  );
}
