import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StatusBar } from "expo-status-bar";
import { useState } from "react";

import { GeistFontLoader } from "@/components/fonts/geist-font-loader";
import { AppShell } from "@/components/layout/app-shell";
import { OnboardingRouter } from "@/components/onboarding/onboarding-router";
import { DaemonProvider } from "@/runtime/daemon-provider";
import { ConversationListProvider } from "@/runtime/conversation-list-provider";
import { ReadinessProvider } from "@/runtime/readiness-provider";
import { ThemeProvider, useResolvedColorScheme, useTheme } from "@/styles/use-theme";

function ThemedStatusBar() {
  const theme = useTheme();
  const colorScheme = useResolvedColorScheme();
  return (
    <StatusBar
      style={colorScheme === "light" ? "dark" : "light"}
      backgroundColor={theme.colors.surface0}
    />
  );
}

export default function RootLayout() {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <GeistFontLoader>
          <DaemonProvider>
            <ConversationListProvider>
              <ReadinessProvider>
                <ThemedStatusBar />
                <OnboardingRouter>
                  <AppShell />
                </OnboardingRouter>
              </ReadinessProvider>
            </ConversationListProvider>
          </DaemonProvider>
        </GeistFontLoader>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
