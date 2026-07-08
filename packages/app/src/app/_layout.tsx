import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StatusBar } from "expo-status-bar";
import { useState } from "react";

import { AppShell } from "@/components/layout/app-shell";
import { DaemonProvider } from "@/runtime/daemon-provider";
import { theme } from "@/styles/theme";

export default function RootLayout() {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <DaemonProvider>
        <StatusBar style="light" backgroundColor={theme.colors.surface0} />
        <AppShell />
      </DaemonProvider>
    </QueryClientProvider>
  );
}
