/**
 * End-to-end proof of the protocol + client spine: spawn the real Rust
 * tamtri-daemon binary against an isolated TAMTRI_HOME, connect with the
 * TypeScript DaemonClient over a live WebSocket, and drive the hello handshake
 * plus a conversation round-trip through the wire protocol.
 *
 * Run with: npm test --workspace @tamtri/client
 */

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";

import { ClientType, method, type ServerInfo } from "@tamtri/protocol";
import { DaemonClient, webSocketTransport } from "../src/index.js";

const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
const daemonBin = join(repoRoot, "target", "debug", "tamtri-daemon");

let home: string;
let daemon: ChildProcess;
let client: DaemonClient;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForEndpoint(portFile: string, tokenFile: string): Promise<{ port: number; token: string }> {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (existsSync(portFile) && existsSync(tokenFile)) {
      const port = Number.parseInt(readFileSync(portFile, "utf8").trim(), 10);
      const token = readFileSync(tokenFile, "utf8").trim();
      if (Number.isFinite(port) && port > 0 && token.length > 0) return { port, token };
    }
    await sleep(100);
  }
  throw new Error("daemon did not publish its endpoint files in time");
}

before(async () => {
  assert.ok(existsSync(daemonBin), `daemon binary missing at ${daemonBin}; run: cargo build -p tamtri-daemon`);
  home = mkdtempSync(join(tmpdir(), "tamtri-e2e-"));
  daemon = spawn(daemonBin, [], {
    env: { ...process.env, TAMTRI_HOME: home, TAMTRI_PORT: "0" },
    stdio: "ignore",
  });

  const { port, token } = await waitForEndpoint(join(home, "daemon.port"), join(home, "daemon.token"));
  client = new DaemonClient({
    clientId: "e2e-test",
    clientType: ClientType.Cli,
    transport: webSocketTransport({ url: `ws://127.0.0.1:${port}/ws`, token }),
  });
});

after(() => {
  client?.close();
  daemon?.kill("SIGKILL");
  if (home) rmSync(home, { recursive: true, force: true });
});

test("hello handshake returns server info with a matching protocol version", async () => {
  const info: ServerInfo = await client.connect();
  assert.equal(info.protocol_version, client.protocolVersion);
  assert.ok(info.server_id.length > 0, "server_id should be populated");
});

test("conversation create + list round-trips over the wire", async () => {
  const created = await client.request<{ id: string; title: string }>(method.CONVERSATION_CREATE, {
    title: "Spine smoke test",
    harness_id: "mock-acp",
    model_id: "mock",
  });
  assert.ok(created.id.length > 0, "created conversation should have an id");

  const list = await client.request<Array<{ id: string; title: string }>>(method.CONVERSATION_LIST);
  assert.ok(
    list.some((c) => c.id === created.id),
    "created conversation should appear in the list",
  );
});

test("unknown method rejects with a JSON-RPC error", async () => {
  await assert.rejects(() => client.request("does.not.exist"), /not found|method/i);
});
