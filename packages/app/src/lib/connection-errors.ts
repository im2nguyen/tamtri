import { isDesktopHost } from "@/constants/layout";
import { isDaemonTokenConfigured, isNativeMobile } from "@/runtime/connection-config";

/** True when running the packaged desktop shell (not Metro dev). */
export function isPackagedApp(): boolean {
  return isDesktopHost() && !__DEV__;
}

export interface ConnectionErrorPresentation {
  title: string;
  hint: string;
}

export function presentConnectionError(raw: string): ConnectionErrorPresentation {
  const message = raw.trim() || "Could not connect.";
  const lower = message.toLowerCase();
  const timedOut = lower.includes("timed out");

  if (isPackagedApp()) {
    return {
      title: timedOut ? "tamtri is taking too long to start" : "Could not reach tamtri",
      hint: timedOut
        ? "Quit tamtri completely and open it again. If this keeps happening, restart your Mac and try once more."
        : "Quit tamtri completely and open it again. The background service may not have started.",
    };
  }

  if (isNativeMobile()) {
    return {
      title: "Could not reach tamtri host",
      hint:
        "On iPhone, run pnpm run dev:ios from the repo root on your Mac (same Wi-Fi). Or open Connect host in the sidebar to paste the LAN WebSocket URL and token manually.",
    };
  }

  if (lower.includes("websocket") || lower.includes("econnrefused") || lower.includes("failed")) {
    const missingToken = !isDaemonTokenConfigured();
    return {
      title: "Could not reach tamtri host",
      hint: __DEV__
        ? missingToken
          ? "The daemon auth token is missing. Stop Metro and run pnpm run dev:web from the repo root (starts the daemon and web app together)."
          : "The daemon is not running. Stop Metro and run pnpm run dev:web from the repo root, or pnpm run dev:desktop for Electron."
        : "Start tamtri from the desktop app, or contact support if the problem continues.",
    };
  }

  return {
    title: "Could not reach tamtri host",
    hint: __DEV__
      ? "Make sure tamtri-daemon is running. Desktop: pnpm run dev:desktop. Browser: pnpm run dev:web."
      : "Restart tamtri and try again.",
  };
}
