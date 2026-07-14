import {
  DaemonClient,
  relayTransport,
  webSocketTransport,
  type DaemonTransportFactory,
} from "@tamtri/client";
import { ClientType } from "@tamtri/protocol";
import { Platform } from "react-native";

import { isDesktopHost } from "@/constants/layout";
import { createDesktopTransport } from "@/desktop/transport";
import {
  defaultDirectConfig,
  isNativeMobile,
  loadConnectionConfig,
  type StoredConnectionConfig,
} from "@/runtime/connection-config";

let client: DaemonClient | undefined;

function resolveClientMeta(): { clientId: string; clientType: ClientType } {
  if (isDesktopHost()) {
    return { clientId: "tamtri-desktop", clientType: ClientType.Desktop };
  }
  if (isNativeMobile()) {
    return { clientId: `tamtri-mobile-${Platform.OS}`, clientType: ClientType.Mobile };
  }
  return { clientId: "tamtri-web", clientType: ClientType.Browser };
}

function transportFromConfig(config: StoredConnectionConfig): DaemonTransportFactory {
  if (config.mode === "relay") {
    return relayTransport({ offer: config.offerInput });
  }
  return webSocketTransport({
    url: config.wsUrl,
    token: config.token,
  });
}

async function buildTransport(): Promise<DaemonTransportFactory> {
  if (isDesktopHost()) {
    return createDesktopTransport();
  }
  const stored = await loadConnectionConfig();
  if (stored) {
    return transportFromConfig(stored);
  }
  const fallback = defaultDirectConfig();
  return webSocketTransport({
    url: fallback.wsUrl,
    token: fallback.token,
  });
}

export async function getDaemonClient(): Promise<DaemonClient> {
  if (client) return client;

  const meta = resolveClientMeta();
  client = new DaemonClient({
    clientId: meta.clientId,
    clientType: meta.clientType,
    appVersion: "0.1.0",
    transport: await buildTransport(),
    connectTimeoutMs: 20_000,
  });

  await client.connect();
  return client;
}

export function resetDaemonClient(): void {
  client?.close();
  client = undefined;
}

export async function reconnectDaemonClient(): Promise<DaemonClient> {
  resetDaemonClient();
  return getDaemonClient();
}
