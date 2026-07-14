#!/usr/bin/env node
/**
 * Connection smoke test: daemon handshake or bounded failure within 20s.
 * Usage: node scripts/connection-smoke.mjs [--no-daemon]
 *
 * Exit 0 on successful hello handshake.
 * Exit 1 on timeout or connection error (with message on stderr).
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const TIMEOUT_MS = 20_000;
const TAMTRI_DIR = join(homedir(), ".tamtri");

function readRuntimeFile(name) {
  try {
    return readFileSync(join(TAMTRI_DIR, name), "utf8").trim();
  } catch {
    return null;
  }
}

async function smokeTest() {
  const port = readRuntimeFile("daemon.port");
  const token = readRuntimeFile("daemon.token");
  if (!port || !token) {
    console.error("connection-smoke: missing ~/.tamtri/daemon.port or daemon.token");
    console.error("Start the daemon first: pnpm run dev:web or pnpm run dev:desktop");
    process.exit(1);
  }

  const wsUrl = `ws://127.0.0.1:${port}/ws?token=${encodeURIComponent(token)}`;
  const started = Date.now();

  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let settled = false;

    const fail = (message) => {
      if (settled) return;
      settled = true;
      socket.close();
      reject(new Error(message));
    };

    const timer = setTimeout(() => {
      fail(`Connection timed out after ${TIMEOUT_MS}ms`);
    }, TIMEOUT_MS);

    socket.addEventListener("open", () => {
      socket.send(
        JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "hello",
          params: {
            client_id: "connection-smoke",
            client_type: "cli",
            protocol_version: 1,
          },
        }),
      );
    });

    socket.addEventListener("message", (event) => {
      try {
        const frame = JSON.parse(String(event.data));
        if (frame.id === 1) {
          settled = true;
          clearTimeout(timer);
          socket.close();
          if (frame.error) {
            reject(new Error(frame.error.message ?? "hello failed"));
            return;
          }
          const elapsed = Date.now() - started;
          console.log(`connection-smoke: hello ok in ${elapsed}ms`);
          resolve(frame.result);
        }
      } catch (err) {
        fail(err instanceof Error ? err.message : String(err));
      }
    });

    socket.addEventListener("error", () => {
      fail(`WebSocket connection to ws://127.0.0.1:${port}/ws failed`);
    });
  });
}

if (process.argv.includes("--no-daemon")) {
  console.log("connection-smoke: --no-daemon skipped (manual negative test)");
  process.exit(0);
}

try {
  await smokeTest();
  process.exit(0);
} catch (err) {
  console.error(`connection-smoke: ${err instanceof Error ? err.message : String(err)}`);
  await delay(0);
  process.exit(1);
}
