#!/usr/bin/env node
/**
 * Web dev loop: ensure tamtri-daemon is running, publish credentials, start Expo web.
 */

import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  ensureDaemon,
  writeAppEnv,
  writeDevEndpoint,
} from "./lib/daemon-dev.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const appDir = join(repoRoot, "packages/app");
const tamtriHome = process.env.TAMTRI_HOME ?? join(homedir(), ".tamtri");
const requestedPort = process.env.TAMTRI_PORT ?? "8377";
const daemonBin =
  process.env.TAMTRI_DAEMON_BIN ??
  join(repoRoot, "target", "debug", "tamtri-daemon");

let daemonChild = null;
let spawnedDaemon = false;
let shuttingDown = false;

function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  if (spawnedDaemon && daemonChild) {
    daemonChild.kill("SIGTERM");
    setTimeout(() => daemonChild?.kill("SIGKILL"), 1500);
  }
  process.exit(code);
}

process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));

let session;
try {
  session = await ensureDaemon({
    repoRoot,
    tamtriHome,
    requestedPort,
    daemonBin,
    bindHost: "127.0.0.1",
  });
} catch (err) {
  console.error(err instanceof Error ? err.message : String(err));
  process.exit(1);
}

daemonChild = session.child;
spawnedDaemon = session.spawned;

if (spawnedDaemon && daemonChild) {
  daemonChild.on("exit", (code) => {
    if (!shuttingDown) {
      console.error(`tamtri-daemon exited (${code ?? "signal"})`);
      shutdown(code ?? 1);
    }
  });
}

const { port, token } = session.endpoint;
const wsUrl = `ws://127.0.0.1:${port}/ws`;
writeAppEnv(appDir, wsUrl, token);
writeDevEndpoint(appDir, wsUrl, token);

console.log(`Daemon ready at ${wsUrl}`);

const expo = spawn("pnpm", ["exec", "expo", "start", "--web", "--clear"], {
  cwd: appDir,
  env: {
    ...process.env,
    EXPO_PUBLIC_DAEMON_WS_URL: wsUrl,
    EXPO_PUBLIC_DAEMON_TOKEN: token,
  },
  stdio: "inherit",
});
expo.on("exit", (code) => shutdown(code ?? 0));
