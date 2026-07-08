/**
 * Daemon lifecycle for the desktop shell. Owns the Rust tamtri-daemon: locates
 * the bundled binary, spawns/supervises it, reads the ~/.tamtri token+port
 * endpoint files, and reports health to the renderer.
 *
 * Structural stub. The Electron build-out step wires child_process spawn,
 * endpoint-file polling, and restart-on-crash.
 */

export interface DaemonEndpoint {
  port: number;
  token: string;
}

export interface DaemonManager {
  start(): Promise<DaemonEndpoint>;
  stop(): Promise<void>;
  status(): Promise<"running" | "stopped">;
}
