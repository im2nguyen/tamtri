import { MessageCircleQuestion } from "lucide-react-native";
import { useState } from "react";
import { ActivityIndicator, Linking, Text, TextInput, View } from "react-native";

import { Button } from "@/components/ui/button";
import type { PendingElicitation } from "@/lib/elicitation";
import { useTheme } from "@/styles/use-theme";

interface ElicitationCardProps {
  elicitation: PendingElicitation;
  responding?: boolean;
  onRespond: (action: "accept" | "decline" | "cancel", dataJson?: string) => void;
}

export function ElicitationCard({ elicitation, responding, onRespond }: ElicitationCardProps) {
  const theme = useTheme();
  const [formJson, setFormJson] = useState("{}");

  return (
    <View
      style={{
        borderWidth: 1,
        borderColor: theme.colors.accent,
        borderRadius: theme.radius.xl,
        backgroundColor: theme.colors.surface2,
        padding: theme.spacing[4],
        gap: theme.spacing[3],
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
        <MessageCircleQuestion color={theme.colors.accentBright} size={18} />
        <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "700" }}>
          INPUT NEEDED
        </Text>
      </View>

      <View>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>From</Text>
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "600", marginTop: 2 }}>
          {elicitation.serverId}
        </Text>
      </View>

      <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, lineHeight: 22 }}>
        {elicitation.message}
      </Text>

      {elicitation.mode === "url" && elicitation.url ? (
        <Button
          label="Open trusted URL"
          variant="secondary"
          compact
          onPress={() => void Linking.openURL(elicitation.url!)}
        />
      ) : null}

      {elicitation.mode === "form" ? (
        <TextInput
          value={formJson}
          onChangeText={setFormJson}
          multiline
          placeholder='{"field": "value"}'
          placeholderTextColor={theme.colors.foregroundMuted}
          style={{
            minHeight: 80,
            borderWidth: 1,
            borderColor: theme.colors.border,
            borderRadius: theme.radius.md,
            padding: theme.spacing[3],
            color: theme.colors.foreground,
            fontFamily: theme.fontFamily.mono,
            fontSize: theme.fontSize.xs,
            backgroundColor: theme.colors.surface0,
          }}
        />
      ) : null}

      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2], alignItems: "center" }}>
        {responding ? <ActivityIndicator color={theme.colors.accentBright} size="small" /> : null}
        <Button
          label="Accept"
          compact
          disabled={responding}
          onPress={() => onRespond("accept", elicitation.mode === "form" ? formJson : undefined)}
        />
        <Button
          label="Decline"
          variant="secondary"
          compact
          disabled={responding}
          onPress={() => onRespond("decline")}
        />
        <Button
          label="Cancel"
          variant="destructive"
          compact
          disabled={responding}
          onPress={() => onRespond("cancel")}
        />
      </View>
    </View>
  );
}
