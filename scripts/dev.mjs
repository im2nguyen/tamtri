#!/usr/bin/env node
/**
 * Full dev loop: one daemon, one Metro (web + iOS LAN), and the Electron shell.
 *
 * - Web:     http://localhost:8081
 * - Desktop: Electron window (IPC bridge to the same daemon)
 * - iOS:     Scan the Expo QR code in Expo Go (same Wi-Fi as this Mac)
 */

import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  ensureDaemon,
  getLanIp,
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

function run(label, cmd, args, env = process.env) {
  const child = spawn(cmd, args, { cwd: repoRoot, env, stdio: "inherit" });
  child.on("exit", (code, signal) => {
    if (signal) console.log(`${label} stopped (${signal})`);
    else if (code && code !== 0) console.log(`${label} exited with code ${code}`);
  });
  return child;
}

let daemonChild = null;
let spawnedDaemon = false;
let shuttingDown = false;
const children = [];

function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  for (const child of children) {
    child.kill("SIGTERM");
  }
  if (spawnedDaemon && daemonChild) {
    daemonChild.kill("SIGTERM");
    setTimeout(() => daemonChild?.kill("SIGKILL"), 1500);
  }
  setTimeout(() => process.exit(code), 300);
}

process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));

const lanIp = getLanIp();
if (!lanIp) {
  console.warn(
    "No LAN IPv4 address detected. iOS dev over Wi-Fi may not work; web and desktop will still start.",
  );
}

let session;
try {
  session = await ensureDaemon({
    repoRoot,
    tamtriHome,
    requestedPort,
    daemonBin,
    bindHost: "0.0.0.0",
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
const localhostWsUrl = `ws://127.0.0.1:${port}/ws`;
const lanWsUrl = lanIp ? `ws://${lanIp}:${port}/ws` : localhostWsUrl;

writeAppEnv(appDir, lanWsUrl, token);
writeDevEndpoint(appDir, lanWsUrl, token, { localhostWsUrl });

console.log("");
console.log("=== tamtri dev ===");
console.log(`Daemon:  ${localhostWsUrl} (LAN: ${lanWsUrl})`);
console.log("Web:     http://localhost:8081");
console.log("Desktop: Electron window opens after Metro starts");
if (lanIp) {
  console.log("iOS:     Scan the QR code below in Expo Go (same Wi-Fi)");
} else {
  console.log("iOS:     Connect this Mac to Wi-Fi, then restart pnpm run dev");
}
console.log("");

const expoEnv = {
  ...process.env,
  EXPO_PUBLIC_DAEMON_WS_URL: lanWsUrl,
  EXPO_PUBLIC_DAEMON_TOKEN: token,
};

const expo = spawn("pnpm", ["exec", "expo", "start", "--web", "--lan", "--clear"], {
  cwd: appDir,
  env: expoEnv,
  stdio: "inherit",
});
children.push(expo);

await new Promise((resolve) => setTimeout(resolve, 2500));

const desktop = run(
  "electron",
  "pnpm",
  ["--filter", "@tamtri/desktop", "run", "start"],
  {
    ...process.env,
    TAMTRI_USE_DEV_SERVER: "1",
    TAMTRI_REUSE_DAEMON: "1",
  },
);
children.push(desktop);

expo.on("exit", (code) => {
  if (!shuttingDown) shutdown(code ?? 0);
});
desktop.on("exit", () => {
  if (!shuttingDown) shutdown(0);
});
