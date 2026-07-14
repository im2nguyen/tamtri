import { useRouter } from "expo-router";
import { Bot, CheckCircle2, ClipboardCopy, ExternalLink } from "lucide-react-native";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ActivityIndicator,
  AppState,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  Text,
  View,
} from "react-native";

import { Button } from "@/components/ui/button";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { onboardingCopy } from "@/content/onboarding-copy";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useHarnessHealth } from "@/hooks/use-harness-health";
import { useReadiness } from "@/hooks/use-readiness";
import { useOnboardingStore } from "@/stores/onboarding-store";
import { useTheme } from "@/styles/use-theme";

const RECHECK_DEBOUNCE_MS = 800;

export function GateScreen() {
  const theme = useTheme();
  const router = useRouter();
  const setPhase = useOnboardingStore((s) => s.setPhase);
  const markItRequestSent = useOnboardingStore((s) => s.markItRequestSent);
  const itRequestSentAt = useOnboardingStore((s) => s.it_request_sent_at);
  const { checklist } = useHarnessHealth();
  const { readyCount, recommendedEntry, recommendation, loading, refresh } = useReadiness();
  const [refreshing, setRefreshing] = useState(false);
  const recheckTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const copy = onboardingCopy.gate;

  const debouncedRefresh = useCallback(() => {
    if (recheckTimer.current) clearTimeout(recheckTimer.current);
    recheckTimer.current = setTimeout(() => {
      void refresh();
    }, RECHECK_DEBOUNCE_MS);
  }, [refresh]);

  useEffect(() => {
    const sub = AppState.addEventListener("change", (state) => {
      if (state === "active") debouncedRefresh();
    });
    return () => {
      sub.remove();
      if (recheckTimer.current) clearTimeout(recheckTimer.current);
    };
  }, [debouncedRefresh]);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await refresh();
    } finally {
      setRefreshing(false);
    }
  };

  const copyChecklist = async () => {
    if (!checklist) return;
    if (Platform.OS === "web" && typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(checklist);
      markItRequestSent();
    }
  };

  const continueToStarter = () => {
    setPhase("starter");
  };

  const recommended = recommendedEntry;
  const showReady = readyCount > 0 && recommended;

  return (
    <View style={{ flex: 1, backgroundColor: theme.colors.surface0 }}>
      <TitlebarDragRegion />
      <ScrollView contentContainerStyle={{ padding: theme.spacing[6], alignItems: "center" }}>
        <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[5] }}>
          <View style={{ gap: theme.spacing[2] }}>
            <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.lg, fontWeight: "700" }}>
              {copy.title}
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, lineHeight: 22 }}>{copy.subtitle}</Text>
          </View>

          {loading ? (
            <ActivityIndicator color={theme.colors.accentBright} />
          ) : showReady ? (
            <View
              style={{
                padding: theme.spacing[4],
                borderRadius: theme.radius.xl,
                borderWidth: 1,
                borderColor: theme.colors.accent,
                backgroundColor: theme.colors.surface2,
                gap: theme.spacing[3],
              }}
            >
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "700" }}>
                {copy.recommendedTitle.toUpperCase()}
              </Text>
              <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[3] }}>
                <View
                  style={{
                    width: 44,
                    height: 44,
                    borderRadius: theme.radius.lg,
                    backgroundColor: theme.colors.surface1,
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <Bot color={theme.colors.accentBright} size={22} />
                </View>
                <View style={{ flex: 1 }}>
                  <Text style={{ color: theme.colors.foreground, fontWeight: "700", fontSize: theme.fontSize.base }}>
                    {recommended.display_name}
                  </Text>
                  <View style={{ flexDirection: "row", alignItems: "center", gap: 6, marginTop: 4 }}>
                    <CheckCircle2 color={theme.colors.accentBright} size={14} />
                    <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>Available</Text>
                  </View>
                  {recommendation?.message ? (
                    <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, marginTop: 4 }}>
                      {recommendation.message}
                    </Text>
                  ) : null}
                </View>
              </View>
              <Button label={copy.continueLabel} onPress={continueToStarter} />
            </View>
          ) : (
            <View
              style={{
                padding: theme.spacing[4],
                borderRadius: theme.radius.xl,
                borderWidth: 1,
                borderColor: theme.colors.border,
                backgroundColor: theme.colors.surface1,
                gap: theme.spacing[3],
              }}
            >
              <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{copy.noAgentsTitle}</Text>
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
                {copy.noAgentsBody}
              </Text>
              {recommended && recommended.status !== "ready" ? (
                <View style={{ gap: theme.spacing[2] }}>
                  <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{recommended.display_name}</Text>
                  {recommended.install_doc_url ? (
                    <Pressable
                      onPress={() => void Linking.openURL(recommended.install_doc_url)}
                      style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[1] }}
                    >
                      <ExternalLink color={theme.colors.accentBright} size={14} />
                      <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>
                        {copy.installGuideLabel}
                      </Text>
                    </Pressable>
                  ) : null}
                </View>
              ) : null}
              <Button
                label={refreshing ? "Checking…" : copy.refreshLabel}
                variant="secondary"
                onPress={() => void handleRefresh()}
                disabled={refreshing}
              />
            </View>
          )}

          <View
            style={{
              padding: theme.spacing[4],
              borderRadius: theme.radius.lg,
              borderWidth: 1,
              borderColor: theme.colors.border,
              backgroundColor: theme.colors.surface1,
              gap: theme.spacing[3],
            }}
          >
            <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>{copy.itTitle}</Text>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 20 }}>
              {copy.itBody}
            </Text>
            {Platform.OS === "web" ? (
              <Pressable
                onPress={() => void copyChecklist()}
                style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2], alignSelf: "flex-start" }}
              >
                <ClipboardCopy color={theme.colors.accentBright} size={16} />
                <Text style={{ color: theme.colors.accentBright, fontWeight: "600" }}>
                  {itRequestSentAt ? copy.itSentLabel : copy.itCopyLabel}
                </Text>
              </Pressable>
            ) : null}
          </View>

          <Pressable onPress={() => router.push("/settings/providers")}>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>{copy.advancedLink}</Text>
          </Pressable>
        </View>
      </ScrollView>
    </View>
  );
}
