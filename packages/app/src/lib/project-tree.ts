import type { ConversationSummaryDto, ProjectDto } from "@tamtri/protocol";

export const UNFILED_PROJECT_ID = "74616d74-7269-7000-0000-000000000001";

export interface ProjectTreeNode {
  id: string;
  name: string;
  project: ProjectDto | null;
  conversations: ConversationSummaryDto[];
  isUnfiled: boolean;
}

export function buildProjectTree(
  projects: ProjectDto[],
  conversations: ConversationSummaryDto[],
): ProjectTreeNode[] {
  const projectIds = new Set(projects.map((project) => project.id));
  const conversationsByProject = new Map<string, ConversationSummaryDto[]>();
  const unfiled: ConversationSummaryDto[] = [];

  for (const conversation of conversations) {
    const projectId = conversation.project_id;
    if (
      !projectId ||
      projectId === UNFILED_PROJECT_ID ||
      !projectIds.has(projectId)
    ) {
      unfiled.push(conversation);
      continue;
    }
    const rows = conversationsByProject.get(projectId) ?? [];
    rows.push(conversation);
    conversationsByProject.set(projectId, rows);
  }

  let hasDaemonUnfiled = false;
  const nodes: ProjectTreeNode[] = projects.flatMap((project) => {
    const isUnfiled = project.id === UNFILED_PROJECT_ID;
    if (isUnfiled) hasDaemonUnfiled = true;
    if (isUnfiled && unfiled.length === 0) return [];
    return [{
      id: project.id,
      name: project.name,
      project,
      conversations: isUnfiled
        ? unfiled
        : (conversationsByProject.get(project.id) ?? []),
      isUnfiled,
    }];
  });

  if (unfiled.length > 0 && !hasDaemonUnfiled) {
    nodes.push({
      id: UNFILED_PROJECT_ID,
      name: "Unfiled",
      project: null,
      conversations: unfiled,
      isUnfiled: true,
    });
  }

  return nodes;
}
