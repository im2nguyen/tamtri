export interface HomeRestorationConversation {
  id: string;
  project_id?: string | null;
}

export type HomeRestorationDecision =
  | { action: "stay"; projectId?: string }
  | { action: "redirect"; conversationId: string; projectId?: string };

export function resolveHomeRestoration(input: {
  draftProjectId: string | null;
  validProjectIds: ReadonlySet<string>;
  conversations: ReadonlyArray<HomeRestorationConversation>;
}): HomeRestorationDecision {
  const requestedDraftIsValid =
    input.draftProjectId !== null && input.validProjectIds.has(input.draftProjectId);
  if (requestedDraftIsValid) {
    return { action: "stay", projectId: input.draftProjectId ?? undefined };
  }

  const latest = input.conversations[0];
  if (latest) {
    const projectId =
      latest.project_id && input.validProjectIds.has(latest.project_id)
        ? latest.project_id
        : undefined;
    return { action: "redirect", conversationId: latest.id, projectId };
  }

  return { action: "stay" };
}

export function isValidComposeProjectId(
  projects: ReadonlyArray<{ id: string }>,
  projectId: string | null,
): boolean {
  if (!projectId) return false;
  return projects.some((project) => project.id === projectId);
}
