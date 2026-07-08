export type AppErrorKind =
  | "connection"
  | "conversation_busy"
  | "schema_version"
  | "malformed_vault"
  | "not_found"
  | "harness_missing"
  | "unknown";

export interface ClassifiedError {
  kind: AppErrorKind;
  title: string;
  message: string;
  recovery?: string;
  actionLabel?: string;
}

export function classifyDaemonError(raw: string): ClassifiedError {
  const message = raw.trim() || "Something went wrong.";
  const lower = message.toLowerCase();

  if (lower.includes("conversation is being written")) {
    return {
      kind: "conversation_busy",
      title: "Conversation in use",
      message: "Another tamtri instance is writing to this conversation. Wait for it to finish or cancel the active run.",
      recovery: "If you closed the other window unexpectedly, cancel the run and try again.",
      actionLabel: "Cancel run",
    };
  }
  if (lower.includes("unsupported schema version")) {
    return {
      kind: "schema_version",
      title: "Update required",
      message,
      recovery: "This conversation uses a newer vault format than this build understands. Update tamtri to open it.",
    };
  }
  if (lower.includes("conversation not found") || lower.includes("not found")) {
    return {
      kind: "not_found",
      title: "Conversation missing",
      message: "This conversation is no longer in your vault. It may have been moved or deleted.",
      recovery: "Pick another conversation from the sidebar or import a .tamtri bundle.",
    };
  }
  if (
    lower.includes("malformed") ||
    lower.includes("integrity") ||
    lower.includes("artifact") ||
    lower.includes("vault")
  ) {
    return {
      kind: "malformed_vault",
      title: "Vault issue",
      message,
      recovery: "Reveal the conversation folder in Finder and check messages.jsonl or attachments.",
      actionLabel: "Reveal folder",
    };
  }
  if (lower.includes("harness") && (lower.includes("missing") || lower.includes("not found"))) {
    return {
      kind: "harness_missing",
      title: "Harness unavailable",
      message,
      recovery: "Open Harness health to see install status for configured agents.",
      actionLabel: "Harness health",
    };
  }
  if (lower.includes("connect") || lower.includes("daemon") || lower.includes("websocket")) {
    return {
      kind: "connection",
      title: "Cannot reach daemon",
      message,
      recovery: "Start tamtri-daemon or relaunch the desktop app, then retry.",
    };
  }

  return {
    kind: "unknown",
    title: "Error",
    message,
  };
}
