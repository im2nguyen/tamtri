/**
 * Bootstrap renderer. A minimal "connecting to host" screen that also serves as
 * a live proof of the desktop -> daemon path: it builds a DaemonClient over the
 * IPC bridge, completes the handshake, and shows the daemon identity plus a
 * conversation count. When the Expo app bundle lands, main.ts points the window
 * at it instead and this file becomes the pre-app splash.
 */

import { DaemonClient } from "@tamtri/client";
import {
  ClientType,
  method,
  type ConversationDto,
  type ConversationSummaryDto,
  type NativeSessionSummary,
  type ServerInfo,
} from "@tamtri/protocol";

import { createDesktopTransport } from "../renderer-transport.js";

function set(id: string, text: string): void {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function show(state: "connecting" | "connected" | "error"): void {
  document.body.dataset.state = state;
}

function defaultHarnessForProvider(provider: string): { harnessId: string; modelId: string } {
  if (provider === "codex") {
    return { harnessId: "codex-native", modelId: "default" };
  }
  return { harnessId: "claude-native", modelId: "default" };
}

function renderNativeSessions(
  client: DaemonClient,
  sessions: NativeSessionSummary[],
): void {
  const list = document.getElementById("native-sessions");
  if (!list) return;
  list.replaceChildren();

  if (sessions.length === 0) {
    const empty = document.createElement("p");
    empty.className = "native-empty";
    empty.textContent = "No native sessions found under ~/.claude or ~/.codex.";
    list.appendChild(empty);
    return;
  }

  for (const session of sessions) {
    const row = document.createElement("div");
    row.className = "native-row";

    const meta = document.createElement("div");
    meta.className = "native-meta";
    const title = document.createElement("div");
    title.className = "native-title";
    title.textContent = session.title;
    const detail = document.createElement("div");
    detail.className = "native-detail";
    detail.textContent = `${session.provider} · ${session.cwd ?? "unknown cwd"}`;
    meta.append(title, detail);

    const button = document.createElement("button");
    button.type = "button";
    button.textContent = "Import";
    button.addEventListener("click", () => {
      void importSession(client, session, button);
    });

    row.append(meta, button);
    list.appendChild(row);
  }
}

async function importSession(
  client: DaemonClient,
  session: NativeSessionSummary,
  button: HTMLButtonElement,
): Promise<void> {
  const defaults = defaultHarnessForProvider(session.provider);
  button.disabled = true;
  button.textContent = "Importing…";
  try {
    const imported = await client.request<ConversationDto>(method.SESSIONS_IMPORT, {
      provider: session.provider,
      path: session.path,
      harness_id: defaults.harnessId,
      model_id: defaults.modelId,
    });
    button.textContent = "Imported";
    set("import-status", `Imported “${imported.title}” (${imported.id.slice(0, 8)}…)`);
    const conversations = await client.request<ConversationSummaryDto[]>(method.CONVERSATION_LIST);
    set("conversation-count", `${conversations.length}`);
  } catch (error) {
    button.disabled = false;
    button.textContent = "Import";
    set(
      "import-status",
      error instanceof Error ? error.message : String(error),
    );
  }
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
    const nativeSessions = info.features?.session_import
      ? await client.request<NativeSessionSummary[]>(method.SESSIONS_LIST_NATIVE)
      : [];

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
    renderNativeSessions(client, nativeSessions);
    show("connected");
  } catch (error) {
    set("error-message", error instanceof Error ? error.message : String(error));
    show("error");
  }
}

void main();
