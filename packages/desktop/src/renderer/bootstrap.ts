/**
 * Bootstrap renderer. A minimal "connecting to host" screen that also serves as
 * a live proof of the desktop -> daemon path: it builds a DaemonClient over the
 * IPC bridge, completes the handshake, and shows the daemon identity plus a
 * conversation count. When the Expo app bundle lands, main.ts points the window
 * at it instead and this file becomes the pre-app splash.
 */

import { DaemonClient } from "@tamtri/client";
import { ClientType, method, type ConversationSummaryDto, type ServerInfo } from "@tamtri/protocol";

import { createDesktopTransport } from "../renderer-transport.js";

function set(id: string, text: string): void {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function show(state: "connecting" | "connected" | "error"): void {
  document.body.dataset.state = state;
}

async function main(): Promise<void> {
  show("connecting");
  const client = new DaemonClient({
    clientId: "desktop-bootstrap",
    clientType: ClientType.Desktop,
    transport: createDesktopTransport(),
  });

  try {
    const info: ServerInfo = await client.connect();
    const conversations = await client.request<ConversationSummaryDto[]>(method.CONVERSATION_LIST);

    set("server-id", info.server_id);
    set("server-version", info.version);
    set("protocol-version", info.protocol_version);
    set("conversation-count", `${conversations.length}`);
    const features = info.features ?? {};
    set(
      "features",
      Object.entries(features)
        .filter(([, on]) => on)
        .map(([name]) => name)
        .join(", ") || "none advertised",
    );
    show("connected");
  } catch (error) {
    set("error-message", error instanceof Error ? error.message : String(error));
    show("error");
  }
}

void main();
