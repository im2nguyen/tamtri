import type { TamtriUIMessage } from "@tamtri/protocol";

export const AUTO_FOLLOW_THRESHOLD_PX = 72;

export interface ScrollMetrics {
  contentHeight: number;
  viewportHeight: number;
  offsetY: number;
}

export function distanceFromBottom(metrics: ScrollMetrics): number {
  return Math.max(0, metrics.contentHeight - metrics.viewportHeight - metrics.offsetY);
}

export function isNearTranscriptBottom(
  metrics: ScrollMetrics,
  threshold = AUTO_FOLLOW_THRESHOLD_PX,
): boolean {
  return distanceFromBottom(metrics) <= threshold;
}

function readableName(value: string): string {
  return value
    .replace(/^mcp[_-]?/i, "")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_./:-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

function firstString(input: unknown, keys: string[]): string | undefined {
  if (!input || typeof input !== "object") return undefined;
  const record = input as Record<string, unknown>;
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  return undefined;
}

function compactObject(value: string): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > 76 ? `${normalized.slice(0, 73)}…` : normalized;
}

export interface ActivitySummary {
  verb: string;
  object?: string;
  status: "running" | "completed" | "failed";
  label: string;
}

export function summarizeToolActivity(input: {
  toolName: string;
  toolInput?: unknown;
  state?: string;
  errorText?: string;
}): ActivitySummary {
  const name = readableName(input.toolName) || "tool";
  const status =
    input.errorText || input.state === "output-error"
      ? "failed"
      : input.state === "input-streaming" || input.state === "input-available"
        ? "running"
        : "completed";
  const verb =
    /\b(read|open|load|get)\b/.test(name)
      ? "Read"
      : /\b(search|find|query|list)\b/.test(name)
        ? "Searched"
        : /\b(fetch|download|request)\b/.test(name)
          ? "Fetched"
          : /\b(write|create|save|add)\b/.test(name)
            ? "Created"
            : /\b(edit|update|patch|replace)\b/.test(name)
              ? "Updated"
              : /\b(delete|remove)\b/.test(name)
                ? "Removed"
                : /\b(run|execute|command|shell)\b/.test(name)
                  ? "Ran"
                  : status === "running"
                    ? "Using"
                    : "Used";
  const objectValue = firstString(input.toolInput, [
    "path",
    "file",
    "filename",
    "uri",
    "url",
    "query",
    "command",
    "title",
    "name",
    "server",
  ]);
  const object = objectValue ? compactObject(objectValue) : name;
  return {
    verb,
    object,
    status,
    label: `${verb} ${object}`,
  };
}

export type RightDockTabId = "artifacts" | "apps" | "tasks";

export interface RightDockTab {
  id: RightDockTabId;
  label: string;
  count: number;
}

export interface RightDockState {
  tabs: RightDockTab[];
  apps: Array<{ uri: string; serverId?: string }>;
  tasks: Array<{ taskId: string; title?: string; status: string; resultSummary?: string }>;
}

export function deriveRightDockState(
  messages: ReadonlyArray<TamtriUIMessage>,
  artifactCount: number,
): RightDockState {
  const apps: RightDockState["apps"] = [];
  const tasksById = new Map<string, RightDockState["tasks"][number]>();

  for (const message of messages) {
    for (const part of message.parts) {
      if (part.type === "data-tamtri-app-resource") {
        apps.push({ uri: part.data.uri, serverId: part.data.server_id });
      } else if (part.type === "data-tamtri-task") {
        tasksById.set(part.data.task_id, {
          taskId: part.data.task_id,
          title: part.data.title,
          status: part.data.status,
          resultSummary: part.data.result_summary,
        });
      }
    }
  }

  const tasks = [...tasksById.values()];
  const tabs: RightDockTab[] = [];
  if (artifactCount > 0) tabs.push({ id: "artifacts", label: "Artifacts", count: artifactCount });
  if (apps.length > 0) tabs.push({ id: "apps", label: "Apps", count: apps.length });
  if (tasks.length > 0) tabs.push({ id: "tasks", label: "Tasks", count: tasks.length });
  return { tabs, apps, tasks };
}
