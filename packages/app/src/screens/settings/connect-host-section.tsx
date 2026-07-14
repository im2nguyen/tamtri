import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { ActivityIndicator, Platform, Pressable, Text, TextInput, View } from "react-native";

import { SettingsCard, SettingsSection } from "@/components/settings/settings-section";
import { Button } from "@/components/ui/button";
import {
  defaultDirectConfig,
  isNativeMobile,
  loadConnectionConfig,
  saveConnectionConfig,
  type StoredConnectionConfig,
} from "@/runtime/connection-config";
import { reconnectDaemonClient } from "@/runtime/daemon-client";
import { useTheme } from "@/styles/use-theme";

type Mode = "direct" | "relay";

export function ConnectHostSection() {
  const theme = useTheme();
  const [mode, setMode] = useState<Mode>("direct");
  const [wsUrl, setWsUrl] = useState(() => defaultDirectConfig().wsUrl);
  const [token, setToken] = useState(() => defaultDirectConfig().token);
  const [offerInput, setOfferInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const inputStyle = useMemo(
    () => ({
      borderWidth: 1,
      borderColor: theme.colors.border,
      borderRadius: theme.radius.md,
      padding: theme.spacing[3],
      color: theme.colors.foreground,
      backgroundColor: theme.colors.surface1,
      fontFamily: Platform.select({
        ios: "Menlo",
        android: "monospace",
        default: theme.fontFamily.mono,
      }),
      fontSize: theme.fontSize.sm,
    }),
    [theme],
  );

  useEffect(() => {
    void (async () => {
      const existing = await loadConnectionConfig();
      if (existing?.mode === "direct") {
        setMode("direct");
        setWsUrl(existing.wsUrl);
        setToken(existing.token);
      } else if (existing?.mode === "relay") {
        setMode("relay");
        setOfferInput(existing.offerInput);
      }
      setLoading(false);
    })();
  }, []);

  const connect = useCallback(async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      let config: StoredConnectionConfig;
      if (mode === "direct") {
        const trimmedUrl = wsUrl.trim();
        const trimmedToken = token.trim();
        if (!trimmedUrl || !trimmedToken) {
          throw new Error("WebSocket URL and token are required.");
        }
        config = { mode: "direct", wsUrl: trimmedUrl, token: trimmedToken };
      } else {
        const trimmedOffer = offerInput.trim();
        if (!trimmedOffer) {
          throw new Error("Paste a relay pairing offer URL or JSON.");
        }
        config = { mode: "relay", offerInput: trimmedOffer };
      }

      if (isNativeMobile()) {
        await saveConnectionConfig(config);
      }
      await reconnectDaemonClient();
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [mode, offerInput, token, wsUrl]);

  if (loading) {
    return <ActivityIndicator color={theme.colors.accentBright} />;
  }

  return (
    <SettingsSection title="Connection">
      <SettingsCard>
        <View style={{ gap: theme.spacing[4], padding: theme.spacing[3] }}>
      <View style={{ flexDirection: "row", gap: theme.spacing[2] }}>
        {(["direct", "relay"] as const).map((tab) => (
          <PressableTab key={tab} active={mode === tab} label={tab === "direct" ? "Direct (LAN)" : "Relay"} onPress={() => setMode(tab)} />
        ))}
      </View>

      {mode === "direct" ? (
        <View style={{ gap: theme.spacing[3] }}>
          <LabeledField label="WebSocket URL">
            <TextInput
              value={wsUrl}
              onChangeText={setWsUrl}
              autoCapitalize="none"
              autoCorrect={false}
              placeholder="ws://192.168.1.42:8377/ws"
              placeholderTextColor={theme.colors.foregroundMuted}
              style={inputStyle}
            />
          </LabeledField>
          <LabeledField label="Bearer token">
            <TextInput
              value={token}
              onChangeText={setToken}
              autoCapitalize="none"
              autoCorrect={false}
              placeholder="from ~/.tamtri/daemon.token"
              placeholderTextColor={theme.colors.foregroundMuted}
              style={inputStyle}
            />
          </LabeledField>
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
            Dev shortcut: run pnpm run dev:ios on your Mac. It prints the LAN URL and injects the token automatically.
          </Text>
        </View>
      ) : (
        <View style={{ gap: theme.spacing[3] }}>
          <TextInput
            value={offerInput}
            onChangeText={setOfferInput}
            multiline
            autoCapitalize="none"
            autoCorrect={false}
            placeholder="tamtri://pair#offer=… or JSON from relay.pairing_offer"
            placeholderTextColor={theme.colors.foregroundMuted}
            style={{ ...inputStyle, minHeight: 120, textAlignVertical: "top" }}
          />
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
            Relay E2EE pairing is wired on the client, but relay.tamtri.dev is not deployed yet. Use Direct (LAN) for
            iPhone dev today.
          </Text>
        </View>
      )}

      {error ? <Text style={{ color: theme.colors.destructive }}>{error}</Text> : null}
      {saved ? (
        <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.sm }}>Connected successfully.</Text>
      ) : null}

      <Button label={saving ? "Connecting…" : "Connect"} onPress={() => void connect()} disabled={saving} />

      {Platform.OS === "web" ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
          Web and desktop clients usually connect automatically via pnpm run dev. This section is mainly for physical
          iOS and Android devices.
        </Text>
      ) : null}
        </View>
      </SettingsCard>
    </SettingsSection>
  );
}

function PressableTab({
  active,
  label,
  onPress,
}: {
  active: boolean;
  label: string;
  onPress: () => void;
}) {
  const theme = useTheme();
  return (
    <Pressable
      onPress={onPress}
      style={{
        flex: 1,
        paddingVertical: theme.spacing[3],
        borderRadius: theme.radius.md,
        borderWidth: 1,
        borderColor: active ? theme.colors.accent : theme.colors.border,
        backgroundColor: active ? theme.colors.surface2 : theme.colors.surface1,
        alignItems: "center",
      }}
    >
      <Text style={{ color: theme.colors.foreground, fontWeight: active ? "700" : "500" }}>{label}</Text>
    </Pressable>
  );
}

function LabeledField({ label, children }: { label: string; children: ReactNode }) {
  const theme = useTheme();
  return (
    <View style={{ gap: theme.spacing[2] }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>{label}</Text>
      {children}
    </View>
  );
}
