import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ActivityIndicator, Pressable, Text, View } from "react-native";
import { useRouter } from "expo-router";

import { DaemonClient } from "@tamtri/client";
import type { EventNotification, ServerInfo } from "@tamtri/protocol";

import {
  getDaemonClient,
  reconnectDaemonClient,
  resetDaemonClient,
} from "@/runtime/daemon-client";
import { presentConnectionError } from "@/lib/connection-errors";
import { isNativeMobile, onConnectionConfigChanged } from "@/runtime/connection-config";
import { useTheme } from "@/styles/use-theme";

interface DaemonContextValue {
  client: DaemonClient;
  serverInfo: ServerInfo;
  subscribe: (handler: (event: EventNotification) => void) => () => void;
}

const DaemonContext = createContext<DaemonContextValue | null>(null);

export function DaemonProvider({ children }: { children: ReactNode }) {
  const theme = useTheme();
  const router = useRouter();
  const [client, setClient] = useState<DaemonClient | null>(null);
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(true);
  const [connectionEpoch, setConnectionEpoch] = useState(0);

  const subscribe = useCallback(
    (handler: (event: EventNotification) => void) => {
      if (!client) {
        throw new Error("daemon not connected");
      }
      return client.subscribe(handler);
    },
    [client],
  );

  const contextValue = useMemo<DaemonContextValue | null>(() => {
    if (!client || !serverInfo) return null;
    return { client, serverInfo, subscribe };
  }, [client, serverInfo, subscribe]);

  useEffect(() => {
    return onConnectionConfigChanged(() => {
      setConnectionEpoch((epoch) => epoch + 1);
    });
  }, []);

  useEffect(() => {
    let cancelled = false;
    setConnecting(true);
    setError(null);
    setClient(null);
    setServerInfo(null);
    resetDaemonClient();

    void (async () => {
      try {
        const connected = await getDaemonClient();
        const info = connected.info!;
        if (cancelled) return;
        setClient(connected);
        setServerInfo(info);
      } catch (err) {
        if (!cancelled) {
          setClient(null);
          setServerInfo(null);
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) setConnecting(false);
      }
    })();

    return () => {
      cancelled = true;
      resetDaemonClient();
    };
  }, [connectionEpoch]);

  if (error) {
    const mobile = isNativeMobile();
    const { title, hint } = presentConnectionError(error);
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0, padding: 24 }}>
        <Text style={{ color: theme.colors.destructive, fontSize: theme.fontSize.base, textAlign: "center" }}>
          {title}
        </Text>
        <Text style={{ color: theme.colors.foregroundMuted, marginTop: 8, textAlign: "center" }}>{error}</Text>
        <Text style={{ color: theme.colors.foregroundMuted, marginTop: 16, textAlign: "center", maxWidth: 420, lineHeight: 22 }}>
          {hint}
        </Text>
        {mobile ? (
          <Pressable
            onPress={() => router.push("/settings/connect")}
            style={{
              marginTop: 20,
              paddingHorizontal: 16,
              paddingVertical: 10,
              borderRadius: theme.radius.md,
              backgroundColor: theme.colors.accent,
            }}
          >
            <Text style={{ color: theme.colors.foreground, fontWeight: "600" }}>Connect host</Text>
          </Pressable>
        ) : null}
        <Pressable
          onPress={() => {
            setConnectionEpoch((epoch) => epoch + 1);
          }}
          style={{ marginTop: 12 }}
        >
          <Text style={{ color: theme.colors.accentBright }}>Retry</Text>
        </Pressable>
      </View>
    );
  }

  if (connecting || !contextValue) {
    return (
      <View style={{ flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: theme.colors.surface0 }}>
        <ActivityIndicator color={theme.colors.accentBright} />
        <Text style={{ color: theme.colors.foregroundMuted, marginTop: 12 }}>Connecting to host…</Text>
      </View>
    );
  }

  return <DaemonContext.Provider value={contextValue}>{children}</DaemonContext.Provider>;
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

export { reconnectDaemonClient };
