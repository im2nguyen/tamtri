import { ShieldAlert } from "lucide-react-native";
import { ActivityIndicator, ScrollView, Text, View } from "react-native";

import { Button } from "@/components/ui/button";
import { isDenyOption, permissionSummary, type PendingPermission } from "@/lib/permissions";
import { theme } from "@/styles/theme";

interface PermissionCardProps {
  permission: PendingPermission;
  responding?: boolean;
  onRespond: (optionId: string) => void;
}

function DiffPreview({ oldText, newText }: { oldText?: string; newText?: string }) {
  if (!oldText && !newText) return null;
  return (
    <ScrollView
      style={{
        maxHeight: 160,
        marginTop: theme.spacing[3],
        backgroundColor: theme.colors.surface0,
        borderRadius: theme.radius.md,
        borderWidth: 1,
        borderColor: theme.colors.border,
      }}
    >
      {oldText ? (
        <Text
          style={{
            color: theme.colors.destructive,
            fontFamily: "monospace",
            fontSize: theme.fontSize.xs,
            padding: theme.spacing[2],
          }}
        >
          {oldText.slice(0, 2000)}
        </Text>
      ) : null}
      {newText ? (
        <Text
          style={{
            color: theme.colors.accentBright,
            fontFamily: "monospace",
            fontSize: theme.fontSize.xs,
            padding: theme.spacing[2],
          }}
        >
          {newText.slice(0, 2000)}
        </Text>
      ) : null}
    </ScrollView>
  );
}

export function PermissionCard({ permission, responding, onRespond }: PermissionCardProps) {
  const allowOptions = permission.options.filter((o) => !isDenyOption(o));
  const denyOptions = permission.options.filter(isDenyOption);

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
        <ShieldAlert color={theme.colors.accentBright} size={18} />
        <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "700", letterSpacing: 0.5 }}>
          PERMISSION REQUIRED
        </Text>
      </View>

      <View>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>Who is asking</Text>
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "600", marginTop: 2 }}>
          {permission.harnessDisplayName ?? "Harness"}
        </Text>
      </View>

      <View>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>What</Text>
        <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "600", marginTop: 2 }}>
          {permission.action}
        </Text>
        <Text
          style={{
            color: theme.colors.foreground,
            fontSize: theme.fontSize.sm,
            marginTop: theme.spacing[2],
            fontFamily: permission.detail.type === "command" ? "monospace" : undefined,
          }}
        >
          {permissionSummary(permission.detail)}
        </Text>
        {permission.detail.type === "file_edit" ? (
          <DiffPreview oldText={permission.detail.diff.old_text} newText={permission.detail.diff.new_text} />
        ) : null}
      </View>

      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: theme.spacing[2], alignItems: "center" }}>
        {responding ? <ActivityIndicator color={theme.colors.accentBright} size="small" /> : null}
        {allowOptions.map((option) => (
          <Button
            key={option.id}
            label={option.label}
            compact
            disabled={responding}
            onPress={() => onRespond(option.id)}
          />
        ))}
        {denyOptions.map((option) => (
          <Button
            key={option.id}
            label={option.label}
            variant="destructive"
            compact
            disabled={responding}
            onPress={() => onRespond(option.id)}
          />
        ))}
      </View>
    </View>
  );
}
