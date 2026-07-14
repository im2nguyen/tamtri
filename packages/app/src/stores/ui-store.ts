import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import {
  ARTIFACT_SIDEBAR_WIDTH,
  clampArtifactSidebarWidth,
  clampLeftSidebarWidth,
  SIDEBAR_WIDTH,
} from "@/constants/layout";
import type { ArtifactRef } from "@/lib/artifacts";
import { persistedStorage } from "@/lib/persisted-storage";
import { migrateUiState } from "@/lib/ui-store-migration";

interface UiState {
  sidebarOpen: boolean;
  sidebarWidth: number;
  expandedProjectIds: string[];
  selectedProjectId: string | null;
  draftProjectId: string | null;
  artifactSidebarWidth: number;
  setSidebarOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setSidebarWidth: (width: number) => void;
  toggleProjectExpanded: (projectId: string) => void;
  setSelectedProject: (projectId: string | null) => void;
  beginProjectDraft: (projectId: string) => void;
  clearProjectDraft: () => void;
  setArtifactSidebarWidth: (width: number) => void;
  artifactPreviewConversationId: string | null;
  selectedArtifact: ArtifactRef | null;
  artifactSidebarOpen: boolean;
  openArtifactPreview: (conversationId: string, artifact: ArtifactRef) => void;
  clearArtifactSelection: () => void;
  closeArtifactPreview: () => void;
  toggleArtifactSidebar: () => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarOpen: true,
      sidebarWidth: SIDEBAR_WIDTH,
      expandedProjectIds: [],
      selectedProjectId: null,
      draftProjectId: null,
      artifactSidebarWidth: ARTIFACT_SIDEBAR_WIDTH,
      setSidebarOpen: (open) => set({ sidebarOpen: open }),
      toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
      setSidebarWidth: (width) => set({ sidebarWidth: clampLeftSidebarWidth(width) }),
      toggleProjectExpanded: (projectId) =>
        set((state) => ({
          expandedProjectIds: state.expandedProjectIds.includes(projectId)
            ? state.expandedProjectIds.filter((id) => id !== projectId)
            : [...state.expandedProjectIds, projectId],
        })),
      setSelectedProject: (projectId) => set({ selectedProjectId: projectId }),
      beginProjectDraft: (projectId) =>
        set((state) => ({
          selectedProjectId: projectId,
          draftProjectId: projectId,
          expandedProjectIds: state.expandedProjectIds.includes(projectId)
            ? state.expandedProjectIds
            : [...state.expandedProjectIds, projectId],
        })),
      clearProjectDraft: () => set({ draftProjectId: null }),
      setArtifactSidebarWidth: (width) =>
        set({ artifactSidebarWidth: clampArtifactSidebarWidth(width) }),
      artifactPreviewConversationId: null,
      selectedArtifact: null,
      artifactSidebarOpen: false,
      openArtifactPreview: (conversationId, artifact) =>
        set({
          artifactPreviewConversationId: conversationId,
          selectedArtifact: artifact,
          artifactSidebarOpen: true,
        }),
      clearArtifactSelection: () =>
        set({
          artifactPreviewConversationId: null,
          selectedArtifact: null,
        }),
      closeArtifactPreview: () =>
        set({
          artifactPreviewConversationId: null,
          selectedArtifact: null,
          artifactSidebarOpen: false,
        }),
      toggleArtifactSidebar: () =>
        set((state) => ({ artifactSidebarOpen: !state.artifactSidebarOpen })),
    }),
    {
      name: "tamtri-ui-state",
      version: 2,
      storage: createJSONStorage(() => persistedStorage),
      partialize: (state) => ({
        sidebarWidth: state.sidebarWidth,
        artifactSidebarWidth: state.artifactSidebarWidth,
        expandedProjectIds: state.expandedProjectIds,
        selectedProjectId: state.selectedProjectId,
      }),
      migrate: (persistedState) => migrateUiState(persistedState) as UiState,
    },
  ),
);
