#!/usr/bin/env node
/**
 * Desktop dev loop: Metro (Expo web) + Electron shell. Electron spawns its own
 * daemon and bridges the wire protocol over IPC — no bearer token in the renderer.
 */

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function run(label, cmd, args, env = process.env) {
  const child = spawn(cmd, args, { cwd: repoRoot, env, stdio: "inherit" });
  child.on("exit", (code, signal) => {
    if (signal) console.log(`${label} stopped (${signal})`);
    else if (code && code !== 0) console.log(`${label} exited with code ${code}`);
  });
  return child;
}

const metro = run("metro", "pnpm", ["--filter", "@tamtri/app", "run", "web"]);

// Give Metro a head start before Electron loads localhost:8081.
await new Promise((resolve) => setTimeout(resolve, 2500));

const desktop = run("electron", "pnpm", ["--filter", "@tamtri/desktop", "run", "start"], {
  ...process.env,
  TAMTRI_USE_DEV_SERVER: "1",
});

let shuttingDown = false;
function shutdown() {
  if (shuttingDown) return;
  shuttingDown = true;
  metro.kill("SIGTERM");
  desktop.kill("SIGTERM");
  setTimeout(() => {
    metro.kill("SIGKILL");
    desktop.kill("SIGKILL");
    process.exit(0);
  }, 1500);
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

desktop.on("exit", shutdown);
metro.on("exit", () => {
  if (!shuttingDown) shutdown();
});
