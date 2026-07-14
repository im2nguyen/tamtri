import {
  clampArtifactSidebarWidth,
  clampLeftSidebarWidth,
} from "../constants/layout";

export interface PersistedUiState {
  sidebarWidth?: number;
  artifactSidebarWidth?: number;
  expandedProjectIds?: unknown;
  selectedProjectId?: unknown;
}

export function migrateUiState(persistedState: unknown): PersistedUiState {
  const state =
    persistedState && typeof persistedState === "object"
      ? ({ ...persistedState } as PersistedUiState)
      : {};
  if (typeof state.sidebarWidth === "number") {
    state.sidebarWidth = clampLeftSidebarWidth(state.sidebarWidth);
  }
  if (typeof state.artifactSidebarWidth === "number") {
    state.artifactSidebarWidth = clampArtifactSidebarWidth(state.artifactSidebarWidth);
  }
  state.expandedProjectIds = Array.isArray(state.expandedProjectIds)
    ? [...new Set(state.expandedProjectIds.filter((id): id is string => typeof id === "string"))]
    : [];
  state.selectedProjectId =
    typeof state.selectedProjectId === "string" ? state.selectedProjectId : null;
  return state;
}
