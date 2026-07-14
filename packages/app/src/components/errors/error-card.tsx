import { AlertTriangle, Info } from "lucide-react-native";
import { Text, View } from "react-native";

import { Button } from "@/components/ui/button";
import type { ClassifiedError } from "@/lib/errors";
import { useTheme } from "@/styles/use-theme";

interface ErrorCardProps {
  error: ClassifiedError;
  onAction?: () => void;
  compact?: boolean;
}

export function ErrorCard({ error, onAction, compact }: ErrorCardProps) {
  const theme = useTheme();
  const Icon = error.kind === "unknown" ? Info : AlertTriangle;
  const tone =
    error.kind === "conversation_busy" || error.kind === "schema_version"
      ? theme.colors.destructive
      : theme.colors.accentBright;

  return (
    <View
      style={{
        backgroundColor: theme.colors.surface2,
        borderRadius: theme.radius.xl,
        borderWidth: 1,
        borderColor: tone,
        padding: compact ? theme.spacing[3] : theme.spacing[4],
        gap: theme.spacing[3],
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
        <Icon color={tone} size={18} />
        <Text style={{ color: theme.colors.foreground, fontWeight: "700" }}>{error.title}</Text>
      </View>
      <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, lineHeight: 22 }}>
        {error.message}
      </Text>
      {error.recovery ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
          {error.recovery}
        </Text>
      ) : null}
      {error.actionLabel && onAction ? (
        <View style={{ alignItems: "flex-start" }}>
          <Button label={error.actionLabel} compact variant="secondary" onPress={onAction} />
        </View>
      ) : null}
    </View>
  );
}
