import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { method, type ConversationDto, type ProjectDto, type RootDto } from "@tamtri/protocol";

import { invalidateConversationList } from "@/hooks/conversation-list-invalidation";
import { shellBridge } from "@/lib/shell";
import { useDaemon } from "@/runtime/daemon-provider";

export function useProjects() {
  const { client, serverInfo } = useDaemon();
  const queryClient = useQueryClient();
  const projectsSupported = Boolean(serverInfo.features?.projects);
  const queryKey = ["projects", serverInfo.server_id] as const;
  const query = useQuery({
    queryKey,
    queryFn: () => client.request<ProjectDto[]>(method.PROJECT_LIST),
    enabled: projectsSupported,
  });

  const refresh = useCallback(async () => {
    await query.refetch();
  }, [query]);

  const refreshAll = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey });
  }, [queryClient, serverInfo.server_id]);

  const createProject = useCallback(
    async (name: string) => {
      const project = await client.request<ProjectDto>(method.PROJECT_CREATE, { name });
      await refreshAll();
      invalidateConversationList();
      return project;
    },
    [client, refreshAll],
  );

  const renameProject = useCallback(
    async (id: string, name: string) => {
      const project = await client.request<ProjectDto>(method.PROJECT_UPDATE, { id, name });
      await refreshAll();
      invalidateConversationList();
      return project;
    },
    [client, refreshAll],
  );

  const deleteProject = useCallback(
    async (id: string) => {
      await client.request(method.PROJECT_DELETE, { id });
      await refreshAll();
      invalidateConversationList();
    },
    [client, refreshAll],
  );

  const attachFilesystemRoot = useCallback(
    async (projectId: string) => {
      const shell = shellBridge();
      if (!shell?.pickOpenFile) return null;
      const path = await shell.pickOpenFile({ title: "Add shared project root" });
      if (!path) return null;
      const name = path.split(/[/\\]/).pop() ?? "Root";
      const root = await client.request<RootDto>(method.PROJECT_ROOT_ATTACH, {
        project_id: projectId,
        name,
        uri: path,
        kind: "filesystem",
        scope: "conversation",
      });
      await refreshAll();
      invalidateConversationList();
      return root;
    },
    [client, refreshAll],
  );

  const createConversation = useCallback(
    async (projectId: string, title: string, harnessId: string, modelId: string) =>
      client.request<ConversationDto>(method.PROJECT_CONVERSATION_CREATE, {
        project_id: projectId,
        title,
        harness_id: harnessId,
        model_id: modelId,
      }),
    [client],
  );

  const moveConversationToProject = useCallback(
    async (conversationId: string, projectId: string) => {
      const dto = await client.request<ConversationDto>(method.CONVERSATION_MOVE_PROJECT, {
        conversation_id: conversationId,
        project_id: projectId,
      });
      await refreshAll();
      invalidateConversationList();
      return dto;
    },
    [client, refreshAll],
  );

  const removeProjectRoot = useCallback(
    async (projectId: string, rootId: string) => {
      await client.request(method.PROJECT_ROOT_REMOVE, {
        project_id: projectId,
        root_id: rootId,
      });
      await refreshAll();
    },
    [client, refreshAll],
  );

  return {
    projects: query.data ?? [],
    projectsSupported,
    loading: projectsSupported && query.isLoading,
    error: !projectsSupported
      ? "Update the host to use projects."
      : query.error
      ? query.error instanceof Error
        ? query.error.message
        : String(query.error)
      : null,
    refresh,
    createProject,
    renameProject,
    deleteProject,
    attachFilesystemRoot,
    createConversation,
    moveConversationToProject,
    removeProjectRoot,
    canAttachFilesystemRoot: Boolean(shellBridge()?.pickOpenFile),
  };
}
