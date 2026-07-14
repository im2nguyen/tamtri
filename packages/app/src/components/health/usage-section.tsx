import { ActivityIndicator, Text, View } from "react-native";

import { Button } from "@/components/ui/button";
import type { HarnessUsageView } from "@/hooks/use-harness-usage";
import type { HarnessUsageEntryDto } from "@tamtri/protocol";
import { useTheme } from "@/styles/use-theme";

function toneColor(tone: string, colors: ReturnType<typeof useTheme>["colors"]): string {
  switch (tone) {
    case "warning":
      return "#d4a72c";
    case "danger":
      return colors.destructive;
    case "ok":
    default:
      return colors.accentBright;
  }
}

function UsageBar({
  label,
  utilizationPct,
  tone,
}: {
  label: string;
  utilizationPct: number;
  tone: string;
}) {
  const theme = useTheme();
  const clamped = Math.max(0, Math.min(100, utilizationPct));
  return (
    <View style={{ gap: theme.spacing[1] }}>
      <View style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>{label}</Text>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
          {Math.round(clamped)}%
        </Text>
      </View>
      <View
        style={{
          height: 6,
          borderRadius: theme.radius.full,
          backgroundColor: theme.colors.surface3,
          overflow: "hidden",
        }}
      >
        <View
          style={{
            width: `${clamped}%`,
            height: "100%",
            backgroundColor: toneColor(tone, theme.colors),
          }}
        />
      </View>
    </View>
  );
}

function UsageCard({ usage }: { usage: HarnessUsageEntryDto }) {
  const theme = useTheme();
  const unavailable = usage.status !== "available";
  return (
    <View
      style={{
        padding: theme.spacing[4],
        borderTopWidth: 1,
        borderTopColor: theme.colors.border,
        gap: theme.spacing[3],
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
        <Text style={{ color: theme.colors.foreground, fontWeight: "600", fontSize: theme.fontSize.sm }}>
          {usage.display_name}
        </Text>
        {usage.plan_label ? (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
            {usage.plan_label}
          </Text>
        ) : null}
        <View style={{ flex: 1 }} />
        {unavailable && !usage.error ? (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>Unavailable</Text>
        ) : null}
      </View>
      {usage.error ? (
        <Text style={{ color: theme.colors.destructive, fontSize: theme.fontSize.xs }} numberOfLines={3}>
          {usage.error}
        </Text>
      ) : null}
      {(usage.windows ?? []).map((window) => (
        <UsageBar
          key={window.id}
          label={window.label}
          utilizationPct={window.utilization_pct}
          tone={window.tone}
        />
      ))}
      {(usage.balances ?? []).map((balance) => (
        <View key={balance.id} style={{ flexDirection: "row", justifyContent: "space-between" }}>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>{balance.label}</Text>
          <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.xs }}>
            {balance.unit === "usd" ? `$${balance.remaining.toFixed(2)}` : balance.remaining}
          </Text>
        </View>
      ))}
      {unavailable && !usage.error && (usage.windows ?? []).length === 0 && (usage.balances ?? []).length === 0 ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
          Sign in to this provider's CLI to see quota data.
        </Text>
      ) : null}
    </View>
  );
}

export function UsageSection({
  view,
  onRefresh,
}: {
  view: HarnessUsageView;
  onRefresh: () => void;
}) {
  const theme = useTheme();
  const busy = view.kind === "loading" || (view.kind === "ready" && view.isRefreshing);

  return (
    <View style={{ gap: theme.spacing[3] }}>
      <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
        <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>Usage</Text>
        <Button
          label={busy ? "Refreshing…" : "Refresh"}
          variant="ghost"
          compact
          onPress={() => void onRefresh()}
        />
      </View>

      <View
        style={{
          borderRadius: theme.radius.lg,
          borderWidth: 1,
          borderColor: theme.colors.border,
          backgroundColor: "transparent",
          overflow: "hidden",
        }}
      >
        {view.kind === "loading" ? (
          <View style={{ padding: theme.spacing[6], alignItems: "center" }}>
            <ActivityIndicator color={theme.colors.accentBright} />
          </View>
        ) : null}
        {view.kind === "unavailable" ? (
          <View style={{ padding: theme.spacing[4] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>{view.message}</Text>
          </View>
        ) : null}
        {view.kind === "error" ? (
          <View style={{ padding: theme.spacing[4], gap: theme.spacing[3] }}>
            <Text style={{ color: theme.colors.destructive, fontSize: theme.fontSize.sm }}>{view.message}</Text>
            <Button label="Retry" variant="secondary" compact onPress={() => void onRefresh()} />
          </View>
        ) : null}
        {view.kind === "ready" && view.payload.providers.length === 0 ? (
          <View style={{ padding: theme.spacing[4] }}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
              No usage providers configured. Add Codex or Claude to your roster.
            </Text>
          </View>
        ) : null}
        {view.kind === "ready"
          ? view.payload.providers.map((usage, index) => (
              <View key={usage.provider_id} style={{ borderTopWidth: index === 0 ? 0 : undefined }}>
                <UsageCard usage={usage} />
              </View>
            ))
          : null}
      </View>
    </View>
  );
}
