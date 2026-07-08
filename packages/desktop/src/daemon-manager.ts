/**
 * Daemon lifecycle for the desktop shell. Owns the Rust tamtri-daemon: spawns it
 * against a chosen TAMTRI_HOME, discovers the endpoint it publishes
 * (daemon.port + daemon.token), supervises the process, and shuts it down.
 *
 * Pure Node (no electron import) so it is testable headlessly; main.ts wires it
 * into the Electron app lifecycle.
 */

import { spawn, type ChildProcess } from "node:child_process";
import { EventEmitter } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export interface DaemonEndpoint {
  port: number;
  token: string;
}

export interface DaemonManagerOptions {
  /** Absolute path to the tamtri-daemon binary. */
  binaryPath: string;
  /** TAMTRI_HOME for the daemon (vault + endpoint files). Defaults to ~/.tamtri. */
  home?: string;
  /** Requested port; 0 (default) lets the daemon bind an ephemeral port. */
  port?: number;
  /** Max time to wait for the daemon to publish its endpoint, in ms. */
  startTimeoutMs?: number;
}

type DaemonEvents = {
  exit: [code: number | null, signal: NodeJS.Signals | null];
};

const POLL_INTERVAL_MS = 50;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Spawns and supervises a tamtri-daemon child process. Emits `exit` when the
 * daemon dies so the shell can surface a "host stopped" state and offer restart.
 */
export class DaemonManager extends EventEmitter<DaemonEvents> {
  private child?: ChildProcess;
  private endpoint?: DaemonEndpoint;
  private stopping = false;

  private readonly home: string;
  private readonly port: number;
  private readonly startTimeoutMs: number;

  constructor(private readonly options: DaemonManagerOptions) {
    super();
    this.home = options.home ?? join(homedir(), ".tamtri");
    this.port = options.port ?? 0;
    this.startTimeoutMs = options.startTimeoutMs ?? 10_000;
  }

  /** Spawn the daemon (if not already running) and resolve once it publishes its
   * endpoint. Idempotent: a second call returns the existing endpoint. */
  async start(): Promise<DaemonEndpoint> {
    if (this.child && this.endpoint) return this.endpoint;
    if (!existsSync(this.options.binaryPath)) {
      throw new Error(`tamtri-daemon binary not found at ${this.options.binaryPath}`);
    }

    this.stopping = false;
    this.child = spawn(this.options.binaryPath, [], {
      env: { ...process.env, TAMTRI_HOME: this.home, TAMTRI_PORT: String(this.port) },
      stdio: "ignore",
    });
    this.child.on("exit", (code, signal) => {
      this.child = undefined;
      this.endpoint = undefined;
      if (!this.stopping) this.emit("exit", code, signal);
    });

    this.endpoint = await this.waitForEndpoint();
    return this.endpoint;
  }

  async stop(): Promise<void> {
    this.stopping = true;
    const child = this.child;
    this.child = undefined;
    this.endpoint = undefined;
    if (!child) return;
    await new Promise<void>((resolve) => {
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
        resolve();
      }, 2000);
      child.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
      child.kill("SIGTERM");
    });
  }

  status(): "running" | "stopped" {
    return this.child && this.endpoint ? "running" : "stopped";
  }

  currentEndpoint(): DaemonEndpoint | undefined {
    return this.endpoint;
  }

  private async waitForEndpoint(): Promise<DaemonEndpoint> {
    const portFile = join(this.home, "daemon.port");
    const tokenFile = join(this.home, "daemon.token");
    const deadline = Date.now() + this.startTimeoutMs;

    while (Date.now() < deadline) {
      if (!this.child) throw new Error("daemon exited before publishing its endpoint");
      if (existsSync(portFile) && existsSync(tokenFile)) {
        const port = Number.parseInt(readFileSync(portFile, "utf8").trim(), 10);
        const token = readFileSync(tokenFile, "utf8").trim();
        if (Number.isFinite(port) && port > 0 && token.length > 0) {
          return { port, token };
        }
      }
      await sleep(POLL_INTERVAL_MS);
    }
    throw new Error(`daemon did not publish its endpoint within ${this.startTimeoutMs}ms`);
  }
}
