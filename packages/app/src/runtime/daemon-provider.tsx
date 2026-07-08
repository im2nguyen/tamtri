import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ActivityIndicator, Text, View } from "react-native";

import { DaemonClient } from "@tamtri/client";
import type { EventNotification, ServerInfo } from "@tamtri/protocol";

import { getDaemonClient, resetDaemonClient } from "@/runtime/daemon-client";
import { theme } from "@/styles/theme";

interface DaemonContextValue {
  client: DaemonClient;
  serverInfo: ServerInfo;
  subscribe: (handler: (event: EventNotification) => void) => () => void;
}

const DaemonContext = createContext<DaemonContextValue | null>(null);

export function DaemonProvider({ children }: { children: ReactNode }) {
  const [value, setValue] = useState<DaemonContextValue | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const client = await getDaemonClient();
        const serverInfo = client.info!;
        if (cancelled) return;
        setValue({
          client,
          serverInfo,
          subscribe: (handler) => client.subscribe(handler),
        });
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
      resetDaemonClient();
    };
  }, []);

  if (error) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0, padding: 24 }}>
        <Text style={{ color: theme.colors.destructive, fontSize: theme.fontSize.base, textAlign: "center" }}>
          Could not reach tamtri host
        </Text>
        <Text style={{ color: theme.colors.foregroundMuted, marginTop: 8, textAlign: "center" }}>{error}</Text>
      </View>
    );
  }

  if (!value) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <ActivityIndicator color={theme.colors.accentBright} />
        <Text style={{ color: theme.colors.foregroundMuted, marginTop: 12 }}>Connecting to host…</Text>
      </View>
    );
  }

  return <DaemonContext.Provider value={value}>{children}</DaemonContext.Provider>;
}

export function useDaemon(): DaemonContextValue {
  const ctx = useContext(DaemonContext);
  if (!ctx) throw new Error("useDaemon must be used within DaemonProvider");
  return ctx;
}

export function useDaemonOptional(): DaemonContextValue | null {
  return useContext(DaemonContext);
}

export function useEventSubscription(handler: (event: EventNotification) => void): void {
  const { subscribe } = useDaemon();
  useEffect(() => subscribe(handler), [handler, subscribe]);
}
