#!/usr/bin/env node
/**
 * iOS dev loop for a physical iPhone on the same Wi-Fi as this Mac.
 *
 * Binds the daemon to all interfaces (TAMTRI_BIND=0.0.0.0) so the phone can
 * reach it by LAN IP, then starts Expo in LAN mode with EXPO_PUBLIC_DAEMON_*
 * pointed at that address.
 *
 * Security: only use on a trusted network. The daemon bearer token gates access.
 */

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { networkInterfaces, homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const tamtriHome = process.env.TAMTRI_HOME ?? join(homedir(), ".tamtri");
const port = process.env.TAMTRI_PORT ?? "8377";
const daemonBin =
  process.env.TAMTRI_DAEMON_BIN ??
  join(repoRoot, "target", "debug", "tamtri-daemon");

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function getLanIp() {
  const nets = networkInterfaces();
  for (const entries of Object.values(nets)) {
    for (const net of entries ?? []) {
      if (net.family === "IPv4" && !net.internal) {
        return net.address;
      }
    }
  }
  return null;
}

async function waitForEndpoint(expectedPid, timeoutMs = 15_000) {
  const portFile = join(tamtriHome, "daemon.port");
  const tokenFile = join(tamtriHome, "daemon.token");
  const pidFile = join(tamtriHome, "daemon.pid");
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(portFile) && existsSync(tokenFile) && existsSync(pidFile)) {
      const published = Number.parseInt(readFileSync(portFile, "utf8").trim(), 10);
      const token = readFileSync(tokenFile, "utf8").trim();
      const publishedPid = Number.parseInt(readFileSync(pidFile, "utf8").trim(), 10);
      if (
        publishedPid === expectedPid &&
        Number.isFinite(published) &&
        published > 0 &&
        token.length > 0
      ) {
        return { port: published, token };
      }
    }
    await sleep(50);
  }
  throw new Error(
    `daemon (pid ${expectedPid}) did not publish ~/.tamtri/daemon.port within ${timeoutMs}ms`,
  );
}

function run(cmd, args, env = process.env) {
  return spawn(cmd, args, { cwd: repoRoot, env, stdio: "inherit" });
}

if (!existsSync(daemonBin)) {
  console.error(`tamtri-daemon not found at ${daemonBin}`);
  console.error("Run: pnpm run daemon:build");
  process.exit(1);
}

const lanIp = getLanIp();
if (!lanIp) {
  console.error("Could not detect a LAN IPv4 address. Connect this Mac to Wi-Fi and retry.");
  process.exit(1);
}

console.log(`Starting tamtri-daemon on 0.0.0.0:${port} (LAN reachable at ${lanIp})…`);
const daemon = spawn(daemonBin, [], {
  env: {
    ...process.env,
    TAMTRI_HOME: tamtriHome,
    TAMTRI_PORT: port,
    TAMTRI_BIND: "0.0.0.0",
    TAMTRI_RELAY_DISABLE: "1",
  },
  stdio: "ignore",
});

let shuttingDown = false;
function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  daemon.kill("SIGTERM");
  setTimeout(() => daemon.kill("SIGKILL"), 1500);
  process.exit(code);
}

process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));
daemon.on("exit", (code) => {
  if (!shuttingDown) {
    console.error(`tamtri-daemon exited (${code ?? "signal"})`);
    shutdown(code ?? 1);
  }
});

if (daemon.pid == null) {
  console.error("failed to spawn tamtri-daemon");
  shutdown(1);
}

let endpoint;
try {
  endpoint = await waitForEndpoint(daemon.pid);
} catch (err) {
  console.error(err.message);
  shutdown(1);
}

const wsUrl = `ws://${lanIp}:${endpoint.port}/ws`;

console.log("");
console.log("=== iPhone setup ===");
console.log("1. iPhone and this Mac must be on the same Wi-Fi network.");
console.log("2. Open Expo Go on your iPhone and scan the QR code below.");
console.log(`3. The app will connect to the daemon at ${wsUrl}`);
console.log("   (token is injected via EXPO_PUBLIC_DAEMON_* — no manual paste needed).");
console.log("");
console.log("If connection fails, open Connect host in the sidebar and verify the URL/token.");
console.log("Relay pairing is not available yet; LAN is the supported dev path.");
console.log("");

const expo = run(
  "pnpm",
  ["--filter", "@tamtri/app", "run", "start", "--", "--lan"],
  {
    ...process.env,
    EXPO_PUBLIC_DAEMON_WS_URL: wsUrl,
    EXPO_PUBLIC_DAEMON_TOKEN: endpoint.token,
  },
);

expo.on("exit", (code) => shutdown(code ?? 0));
