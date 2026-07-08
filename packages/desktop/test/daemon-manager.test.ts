/**
 * Headless test for the daemon manager: it spawns the real tamtri-daemon binary
 * against an isolated TAMTRI_HOME, discovers the published endpoint, reports
 * running status, and shuts the process down. No Electron needed.
 *
 * Run with: npm test --workspace @tamtri/desktop
 */

import assert from "node:assert/strict";
import { mkdtempSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";

import { DaemonManager } from "../src/daemon-manager.js";

const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
const daemonBin = join(repoRoot, "target", "debug", "tamtri-daemon");

let home: string;
let manager: DaemonManager;

before(() => {
  assert.ok(existsSync(daemonBin), `daemon binary missing at ${daemonBin}; run: cargo build -p tamtri-daemon`);
  home = mkdtempSync(join(tmpdir(), "tamtri-desktop-"));
  manager = new DaemonManager({ binaryPath: daemonBin, home, port: 0 });
});

after(async () => {
  await manager?.stop();
  if (home) rmSync(home, { recursive: true, force: true });
});

test("start publishes a usable endpoint and reports running", async () => {
  const endpoint = await manager.start();
  assert.ok(endpoint.port > 0, "endpoint should have a bound port");
  assert.ok(endpoint.token.length > 0, "endpoint should have a token");
  assert.equal(manager.status(), "running");
});

test("start is idempotent", async () => {
  const first = manager.currentEndpoint();
  const second = await manager.start();
  assert.deepEqual(second, first);
});

test("stop tears the daemon down", async () => {
  await manager.stop();
  assert.equal(manager.status(), "stopped");
});
