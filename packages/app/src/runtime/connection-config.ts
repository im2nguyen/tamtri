import AsyncStorage from "@react-native-async-storage/async-storage";
import Constants from "expo-constants";
import { Platform } from "react-native";

export type ConnectionMode = "direct" | "relay";

export interface DirectConnectionConfig {
  mode: "direct";
  wsUrl: string;
  token: string;
}

export interface RelayConnectionConfig {
  mode: "relay";
  offerInput: string;
}

export type StoredConnectionConfig = DirectConnectionConfig | RelayConnectionConfig;

const STORAGE_KEY = "@tamtri/connection-config";
const DEV_ENDPOINT_PATH = "/tamtri-dev-endpoint.json";

type ConfigListener = () => void;
const listeners = new Set<ConfigListener>();

let devEndpointCache: DirectConnectionConfig | null | undefined;
let devEndpointFetch: Promise<DirectConnectionConfig | null> | null = null;

export function onConnectionConfigChanged(listener: ConfigListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function notifyChanged(): void {
  for (const listener of listeners) {
    listener();
  }
}

function readExtraDaemonCredentials(): { wsUrl?: string; token?: string } {
  const extra = Constants.expoConfig?.extra as
    | { daemonWsUrl?: string; daemonToken?: string }
    | undefined;
  return {
    wsUrl: extra?.daemonWsUrl?.trim(),
    token: extra?.daemonToken?.trim(),
  };
}

/** Sync resolution from inlined env and expo config extra. */
export function resolveDaemonCredentialsSync(): { wsUrl: string; token: string } | null {
  const extra = readExtraDaemonCredentials();
  const wsUrl = process.env.EXPO_PUBLIC_DAEMON_WS_URL?.trim() || extra.wsUrl;
  const token = process.env.EXPO_PUBLIC_DAEMON_TOKEN?.trim() || extra.token;
  if (!wsUrl || !token) return null;
  return { wsUrl, token };
}

function isWebDev(): boolean {
  return __DEV__ && Platform.OS === "web";
}

async function fetchDevEndpoint(): Promise<DirectConnectionConfig | null> {
  if (!isWebDev() || typeof fetch === "undefined") {
    return null;
  }

  if (devEndpointCache !== undefined) {
    return devEndpointCache;
  }

  if (!devEndpointFetch) {
    devEndpointFetch = (async () => {
      try {
        const response = await fetch(DEV_ENDPOINT_PATH, { cache: "no-store" });
        if (!response.ok) {
          return null;
        }
        const payload = (await response.json()) as {
          wsUrl?: string;
          localhostWsUrl?: string;
          token?: string;
        };
        const wsUrl = (payload.localhostWsUrl ?? payload.wsUrl)?.trim();
        const token = payload.token?.trim();
        if (!wsUrl || !token) {
          return null;
        }
        return { mode: "direct", wsUrl, token };
      } catch {
        return null;
      } finally {
        devEndpointFetch = null;
      }
    })();
  }

  const fetched = await devEndpointFetch;
  devEndpointCache = fetched;
  return fetched;
}

export function isDaemonTokenConfigured(): boolean {
  if (resolveDaemonCredentialsSync()?.token) {
    return true;
  }
  return Boolean(devEndpointCache?.token);
}

function envDirectConfig(): DirectConnectionConfig | null {
  const creds = resolveDaemonCredentialsSync();
  if (!creds) return null;
  return { mode: "direct", wsUrl: creds.wsUrl, token: creds.token };
}

export function isNativeMobile(): boolean {
  return Platform.OS === "ios" || Platform.OS === "android";
}

export async function loadConnectionConfig(): Promise<StoredConnectionConfig | null> {
  if (isNativeMobile()) {
    try {
      const raw = await AsyncStorage.getItem(STORAGE_KEY);
      if (raw) {
        return JSON.parse(raw) as StoredConnectionConfig;
      }
    } catch {
      // fall through to env defaults
    }
  }

  if (isWebDev()) {
    const fetched = await fetchDevEndpoint();
    if (fetched) {
      return fetched;
    }
  }

  return envDirectConfig();
}

export async function saveConnectionConfig(config: StoredConnectionConfig): Promise<void> {
  if (!isNativeMobile()) {
    throw new Error("Connection config is persisted on mobile only; web/desktop use dev scripts.");
  }
  await AsyncStorage.setItem(STORAGE_KEY, JSON.stringify(config));
  notifyChanged();
}

export async function clearConnectionConfig(): Promise<void> {
  if (!isNativeMobile()) return;
  await AsyncStorage.removeItem(STORAGE_KEY);
  notifyChanged();
}

export function defaultDirectConfig(): DirectConnectionConfig {
  const creds = resolveDaemonCredentialsSync();
  const wsUrl = creds?.wsUrl ?? devEndpointCache?.wsUrl ?? "ws://127.0.0.1:8377/ws";
  const token = creds?.token ?? devEndpointCache?.token ?? "";
  if (__DEV__ && !token && typeof console !== "undefined") {
    console.warn(
      "[tamtri] No daemon auth token. From the repo root run: pnpm run dev or pnpm run dev:web.",
    );
  }
  return {
    mode: "direct",
    wsUrl,
    token,
  };
}
