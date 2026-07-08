#!/usr/bin/env node
/**
 * Web dev loop: spawn tamtri-daemon (fixed port 8377), read token from ~/.tamtri,
 * then start Expo web with EXPO_PUBLIC_DAEMON_* set so the browser can connect.
 */

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
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

async function waitForEndpoint(timeoutMs = 15_000) {
  const portFile = join(tamtriHome, "daemon.port");
  const tokenFile = join(tamtriHome, "daemon.token");
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(portFile) && existsSync(tokenFile)) {
      const published = Number.parseInt(readFileSync(portFile, "utf8").trim(), 10);
      const token = readFileSync(tokenFile, "utf8").trim();
      if (Number.isFinite(published) && published > 0 && token.length > 0) {
        return { port: published, token };
      }
    }
    await sleep(50);
  }
  throw new Error(`daemon did not publish ~/.tamtri/daemon.port within ${timeoutMs}ms`);
}

function run(cmd, args, env = process.env) {
  return spawn(cmd, args, { cwd: repoRoot, env, stdio: "inherit" });
}

if (!existsSync(daemonBin)) {
  console.error(`tamtri-daemon not found at ${daemonBin}`);
  console.error("Run: npm run daemon:build");
  process.exit(1);
}

console.log(`Starting tamtri-daemon on port ${port}…`);
const daemon = spawn(daemonBin, [], {
  env: { ...process.env, TAMTRI_HOME: tamtriHome, TAMTRI_PORT: port },
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

let endpoint;
try {
  endpoint = await waitForEndpoint();
} catch (err) {
  console.error(err.message);
  shutdown(1);
}

const wsUrl = `ws://127.0.0.1:${endpoint.port}/ws`;
console.log(`Daemon ready at ${wsUrl}`);

const expo = run("npm", ["run", "web", "--workspace", "@tamtri/app"], {
  ...process.env,
  EXPO_PUBLIC_DAEMON_WS_URL: wsUrl,
  EXPO_PUBLIC_DAEMON_TOKEN: endpoint.token,
});

expo.on("exit", (code) => shutdown(code ?? 0));
