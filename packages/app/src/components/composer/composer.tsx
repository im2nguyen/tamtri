import { ArrowUp } from "lucide-react-native";
import { useState } from "react";
import { ActivityIndicator, Pressable, Text, TextInput, View } from "react-native";

import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { theme } from "@/styles/theme";

interface ComposerProps {
  onSend: (text: string) => Promise<void>;
  disabled?: boolean;
  sending?: boolean;
  placeholder?: string;
}

export function Composer({ onSend, disabled, sending, placeholder = "Message tamtri…" }: ComposerProps) {
  const [text, setText] = useState("");

  const submit = async () => {
    const value = text.trim();
    if (!value || disabled || sending) return;
    setText("");
    await onSend(value);
  };

  return (
    <View
      style={{
        borderTopWidth: 1,
        borderTopColor: theme.colors.border,
        backgroundColor: theme.colors.surfaceWorkspace,
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
          borderColor: theme.colors.borderAccent,
          paddingHorizontal: theme.spacing[3],
          paddingVertical: theme.spacing[2],
        }}
      >
        <TextInput
          value={text}
          onChangeText={setText}
          placeholder={placeholder}
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
          <View style={{ flexDirection: "row", gap: theme.spacing[2] }}>
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
