import { DaemonClient } from "@tamtri/client";
import { webSocketTransport } from "@tamtri/client";
import { ClientType } from "@tamtri/protocol";

import { isDesktopHost } from "@/constants/layout";
import { createDesktopTransport } from "@/desktop/transport";

let client: DaemonClient | undefined;

export async function getDaemonClient(): Promise<DaemonClient> {
  if (client) return client;

  const transport = isDesktopHost()
    ? createDesktopTransport()
    : webSocketTransport({
        url: process.env.EXPO_PUBLIC_DAEMON_WS_URL ?? "ws://127.0.0.1:8377/ws",
        token: process.env.EXPO_PUBLIC_DAEMON_TOKEN ?? "",
      });

  client = new DaemonClient({
    clientId: isDesktopHost() ? "tamtri-desktop" : "tamtri-web",
    clientType: isDesktopHost() ? ClientType.Desktop : ClientType.Browser,
    appVersion: "0.1.0",
    transport,
  });

  await client.connect();
  return client;
}

export function resetDaemonClient(): void {
  client?.close();
  client = undefined;
}
