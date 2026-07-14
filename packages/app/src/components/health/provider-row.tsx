import { Bot, ExternalLink, Package } from "lucide-react-native";
import { Linking, Pressable, Text, View } from "react-native";

import { Disclosure } from "@/components/ui/disclosure";
import { Switch } from "@/components/ui/switch";
import type { HarnessProviderEntry } from "@/hooks/use-harness-providers";
import { useTheme } from "@/styles/use-theme";

type StatusTone = "success" | "warning" | "muted" | "danger";

interface ProviderStatus {
  tone: StatusTone;
  label: string;
  modelCount: number | null;
}

function getProviderStatus(entry: HarnessProviderEntry): ProviderStatus {
  if (!entry.enabled) {
    return { tone: "muted", label: "Disabled", modelCount: null };
  }

  const state = entry.readiness_state ?? entry.status;
  switch (state) {
    case "ready":
      return {
        tone: "success",
        label: "Available",
        modelCount:
          entry.model_count && entry.model_count > 0 ? entry.model_count : null,
      };
    case "sign_in_required":
      return { tone: "warning", label: "Needs sign-in", modelCount: null };
    case "missing":
      return { tone: "warning", label: "Not installed", modelCount: null };
    case "misconfigured":
      return {
        tone: "warning",
        label:
          entry.recovery_action === "fix_roster" ||
          entry.readiness_message?.toLowerCase().includes("adapter")
            ? "Misconfigured"
            : "Not executable",
        modelCount: null,
      };
    case "check_failed":
      return { tone: "danger", label: "Check failed", modelCount: null };
    case "disabled":
      return { tone: "muted", label: "Disabled", modelCount: null };
    case "installed":
      return { tone: "warning", label: "Installed", modelCount: null };
    case "unknown":
      return { tone: "warning", label: "Unknown", modelCount: null };
    default:
      return { tone: "warning", label: "Unavailable", modelCount: null };
  }
}

function statusColor(tone: StatusTone, colors: ReturnType<typeof useTheme>["colors"]): string {
  switch (tone) {
    case "success":
      return colors.accentBright;
    case "warning":
      return "#d4a72c";
    case "danger":
      return colors.destructive;
    default:
      return colors.foregroundMuted;
  }
}

function ProviderIcon({ adapterType }: { adapterType: string }) {
  const theme = useTheme();
  const Icon = adapterType === "native" ? Bot : Package;
  return <Icon color={theme.colors.foreground} size={18} />;
}

function StatusIndicator({ status }: { status: ProviderStatus }) {
  const theme = useTheme();
  return (
    <View style={{ flexDirection: "row", alignItems: "center", gap: 6 }}>
      <View
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          backgroundColor: statusColor(status.tone, theme.colors),
        }}
      />
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
        {status.label}
      </Text>
      {status.modelCount !== null ? (
        <>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>·</Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
            {status.modelCount === 1 ? "1 model" : `${status.modelCount} models`}
          </Text>
        </>
      ) : null}
    </View>
  );
}

interface ProviderRowProps {
  entry: HarnessProviderEntry;
  isFirst: boolean;
  isToggling: boolean;
  onToggleEnabled: (agentId: string, enabled: boolean) => void;
}

export function ProviderRow({ entry, isFirst, isToggling, onToggleEnabled }: ProviderRowProps) {
  const theme = useTheme();
  const status = getProviderStatus(entry);
  const isReady = (entry.readiness_state ?? entry.status) === "ready";
  const detailMessage = entry.readiness_message?.trim();

  return (
    <View
      style={{
        paddingHorizontal: theme.spacing[3],
        paddingVertical: theme.spacing[2],
        borderTopWidth: isFirst ? 0 : 1,
        borderTopColor: theme.colors.border,
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[3] }}>
        <View style={{ flex: 1, minWidth: 0 }}>
          <Disclosure
            accessibilityLabel={`${entry.display_name} tool details`}
            title={
              <View style={{ flex: 1, flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
                <ProviderIcon adapterType={entry.adapter_type} />
                <View style={{ flex: 1, minWidth: 0, gap: theme.spacing[1] }}>
                  <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
                    <Text style={{ color: theme.colors.foreground, fontWeight: "600" }} numberOfLines={1}>
                      {entry.display_name}
                    </Text>
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                      {entry.adapter_type === "native" ? "Native" : "ACP"}
                    </Text>
                  </View>
                  <StatusIndicator status={status} />
                </View>
              </View>
            }
          >
            <View
              style={{
                marginLeft: 30,
                paddingTop: theme.spacing[2],
                paddingBottom: theme.spacing[1],
                gap: theme.spacing[2],
              }}
            >
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                Command: {entry.command}
                {entry.adapter_kind && entry.adapter_kind !== "acp" ? ` (${entry.adapter_kind})` : ""}
              </Text>
              {detailMessage ? (
                <Text
                  style={{
                    color: theme.colors.foregroundMuted,
                    fontSize: theme.fontSize.xs,
                    lineHeight: 18,
                  }}
                >
                  {detailMessage}
                </Text>
              ) : null}
              {!isReady && entry.install_doc_url ? (
                <Pressable
                  onPress={() => void Linking.openURL(entry.install_doc_url)}
                  style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}
                >
                  <ExternalLink color={theme.colors.accentBright} size={12} />
                  <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs }}>
                    {entry.recovery_action === "fix_roster"
                      ? "Roster setup docs"
                      : (entry.readiness_state ?? entry.status) === "sign_in_required"
                        ? "Sign-in guide"
                        : "Install guide"}
                  </Text>
                </Pressable>
              ) : null}
            </View>
          </Disclosure>
        </View>
        <Switch
          value={entry.enabled}
          disabled={isToggling}
          accessibilityLabel={`Enable ${entry.display_name}`}
          onValueChange={(value) => void onToggleEnabled(entry.id, value)}
        />
      </View>
    </View>
  );
}
