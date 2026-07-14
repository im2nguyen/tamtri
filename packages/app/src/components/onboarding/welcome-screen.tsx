import { useRouter } from "expo-router";
import { Sparkles } from "lucide-react-native";
import { useEffect, useState } from "react";
import { ActivityIndicator, Text, View } from "react-native";

import { Button } from "@/components/ui/button";
import { TitlebarDragRegion } from "@/components/desktop/titlebar-drag-region";
import { onboardingCopy } from "@/content/onboarding-copy";
import { MAX_CONTENT_WIDTH } from "@/constants/layout";
import { useConversationList } from "@/hooks/use-conversations";
import { useReadiness } from "@/hooks/use-readiness";
import { isExampleConversation } from "@/lib/conversation-kind";
import { useOnboardingStore } from "@/stores/onboarding-store";
import { useTheme } from "@/styles/use-theme";

export function WelcomeScreen() {
  const theme = useTheme();
  const router = useRouter();
  const setPhase = useOnboardingStore((s) => s.setPhase);
  const completeOnboarding = useOnboardingStore((s) => s.completeOnboarding);
  const { readyCount } = useReadiness();
  const { conversations, loading } = useConversationList();
  const [exampleId, setExampleId] = useState<string | null>(null);
  const copy = onboardingCopy.welcome;

  useEffect(() => {
    const example = conversations.find((row) => isExampleConversation(row));
    setExampleId(example?.id ?? null);
  }, [conversations]);

  const continueFlow = () => {
    if (exampleId) {
      setPhase("gate");
      router.replace(`/conversation/${exampleId}`);
      return;
    }
    setPhase(readyCount > 0 ? "starter" : "gate");
  };

  const skip = () => {
    completeOnboarding();
  };

  if (loading) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <ActivityIndicator color={theme.colors.accentBright} />
      </View>
    );
  }

  return (
    <View
      style={{
        flex: 1,
        backgroundColor: theme.colors.surface0,
        alignItems: "center",
        justifyContent: "center",
        padding: theme.spacing[6],
      }}
    >
      <TitlebarDragRegion />
      <View style={{ width: "100%", maxWidth: MAX_CONTENT_WIDTH, gap: theme.spacing[5] }}>
        <View
          style={{
            width: 56,
            height: 56,
            borderRadius: theme.radius.xl,
            backgroundColor: theme.colors.surface2,
            alignItems: "center",
            justifyContent: "center",
            borderWidth: 1,
            borderColor: theme.colors.borderAccent,
          }}
        >
          <Sparkles color={theme.colors.accentBright} size={28} />
        </View>
        <View style={{ gap: theme.spacing[2] }}>
          <Text style={{ color: theme.colors.foreground, fontSize: 32, fontWeight: "700" }}>{copy.title}</Text>
          <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.lg, lineHeight: 28 }}>
            Turn a spreadsheet into a finished report using the AI tools you already have.
          </Text>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.base, lineHeight: 24, marginTop: theme.spacing[2] }}>
            {copy.body} Local files, no account, no telemetry.
          </Text>
        </View>
        <View style={{ flexDirection: "row", gap: theme.spacing[3], flexWrap: "wrap" }}>
          <Button
            label={exampleId ? "See the example report" : copy.continueLabel}
            onPress={continueFlow}
          />
          <Button label={copy.skipLabel} variant="ghost" onPress={skip} />
        </View>
      </View>
    </View>
  );
}
