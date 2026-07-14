export interface PermissionOption {
  id: string;
  label: string;
}

export interface PermissionDiff {
  path: string;
  change: "created" | "modified" | "deleted";
  old_text?: string;
  new_text?: string;
}

export type PermissionDetail =
  | { type: "file_edit"; diff: PermissionDiff }
  | { type: "command"; command: string }
  | { type: "other"; value: unknown };

export interface PendingPermission {
  requestId: string;
  action: string;
  detail: PermissionDetail;
  options: PermissionOption[];
  harnessDisplayName?: string;
}

export function parsePermissionRequested(payloadJson: string): PendingPermission | null {
  try {
    const raw = JSON.parse(payloadJson) as Record<string, unknown>;
    if (raw.type !== "permission_requested") return null;

    const detailRaw = raw.detail as Record<string, unknown> | undefined;
    let detail: PermissionDetail = { type: "other", value: raw.detail };
    if (detailRaw?.type === "file_edit" && detailRaw.diff) {
      const diff = detailRaw.diff as Record<string, unknown>;
      detail = {
        type: "file_edit",
        diff: {
          path: String(diff.path ?? ""),
          change: (diff.change as PermissionDiff["change"]) ?? "modified",
          old_text: diff.old_text ? String(diff.old_text) : undefined,
          new_text: diff.new_text ? String(diff.new_text) : undefined,
        },
      };
    } else if (detailRaw?.type === "command") {
      detail = { type: "command", command: String(detailRaw.command ?? "") };
    }

    const options = Array.isArray(raw.options)
      ? raw.options.map((opt) => {
          const item = opt as Record<string, unknown>;
          return {
            id: String(item.id ?? ""),
            label: String(item.label ?? item.id ?? "Option"),
          };
        })
      : [];

    return {
      requestId: String(raw.request_id ?? ""),
      action: String(raw.action ?? "Permission request"),
      detail,
      options,
      harnessDisplayName: raw.harness_display_name
        ? String(raw.harness_display_name)
        : undefined,
    };
  } catch {
    return null;
  }
}

export function permissionSummary(detail: PermissionDetail): string {
  switch (detail.type) {
    case "file_edit":
      return `${detail.diff.change} ${detail.diff.path}`;
    case "command":
      return detail.command;
    case "other":
      return typeof detail.value === "string" ? detail.value : JSON.stringify(detail.value, null, 2);
  }
}

export function isDenyOption(option: PermissionOption): boolean {
  const id = option.id.toLowerCase();
  return id === "deny" || id === "reject" || id === "decline";
}
