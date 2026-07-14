const fs = require("fs");
const path = require("path");
const os = require("os");

/** Inject EXPO_PUBLIC daemon URL/token from local env or ~/.tamtri. */
function loadTamtriDaemonEnv() {
  if (process.env.EXPO_PUBLIC_DAEMON_WS_URL && process.env.EXPO_PUBLIC_DAEMON_TOKEN) {
    return;
  }

  const appEnvLocal = path.join(__dirname, ".env.development.local");
  if (fs.existsSync(appEnvLocal)) {
    const raw = fs.readFileSync(appEnvLocal, "utf8");
    for (const line of raw.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eq = trimmed.indexOf("=");
      if (eq <= 0) continue;
      const key = trimmed.slice(0, eq).trim();
      const value = trimmed.slice(eq + 1).trim();
      if (key.startsWith("EXPO_PUBLIC_") && !process.env[key]) {
        process.env[key] = value;
      }
    }
    if (process.env.EXPO_PUBLIC_DAEMON_WS_URL && process.env.EXPO_PUBLIC_DAEMON_TOKEN) {
      return;
    }
  }

  const dir = path.join(os.homedir(), ".tamtri");
  const portPath = path.join(dir, "daemon.port");
  const tokenPath = path.join(dir, "daemon.token");
  const pidPath = path.join(dir, "daemon.pid");

  try {
    if (!fs.existsSync(portPath) || !fs.existsSync(tokenPath)) {
      return;
    }

    const port = fs.readFileSync(portPath, "utf8").trim();
    const token = fs.readFileSync(tokenPath, "utf8").trim();
    if (!port || !token) {
      return;
    }

    if (fs.existsSync(pidPath)) {
      const pid = Number.parseInt(fs.readFileSync(pidPath, "utf8").trim(), 10);
      if (Number.isFinite(pid)) {
        try {
          process.kill(pid, 0);
        } catch {
          console.warn(
            "[tamtri] Stale ~/.tamtri/daemon.pid; using port/token files anyway. Run pnpm run dev:web to refresh.",
          );
        }
      }
    }

    process.env.EXPO_PUBLIC_DAEMON_WS_URL = `ws://127.0.0.1:${port}/ws`;
    process.env.EXPO_PUBLIC_DAEMON_TOKEN = token;
  } catch (err) {
    console.warn("[tamtri] Could not read ~/.tamtri daemon env:", err.message);
  }
}

/** @param {import('expo/config').ConfigContext} ctx */
module.exports = ({ config }) => {
  loadTamtriDaemonEnv();
  return {
    ...config,
    extra: {
      ...config.extra,
      daemonWsUrl: process.env.EXPO_PUBLIC_DAEMON_WS_URL,
      daemonToken: process.env.EXPO_PUBLIC_DAEMON_TOKEN,
    },
  };
};
