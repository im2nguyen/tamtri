import assert from "node:assert/strict";
import { describe, test } from "node:test";
import type { ConversationSummaryDto, ProjectDto } from "@tamtri/protocol";

import {
  buildProjectTree,
  UNFILED_PROJECT_ID,
} from "../src/lib/project-tree";
import { migrateUiState } from "../src/lib/ui-store-migration";

const projects: ProjectDto[] = [
  {
    id: "project-a",
    name: "Alpha",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    roots: [],
  },
  {
    id: "project-b",
    name: "Beta",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    roots: [],
  },
];

function conversation(
  id: string,
  projectId?: string,
): ConversationSummaryDto {
  return {
    id,
    title: id,
    updated_at: "2026-01-01T00:00:00Z",
    project_id: projectId,
    kind: "conversation",
  };
}

describe("project tree projection", () => {
  test("uses the daemon-owned immutable Unfiled project", () => {
    const daemonProjects: ProjectDto[] = [
      {
        id: UNFILED_PROJECT_ID,
        name: "Unfiled",
        created_at: "1970-01-01T00:00:00Z",
        updated_at: "1970-01-01T00:00:00Z",
        roots: [],
      },
      ...projects,
    ];
    const tree = buildProjectTree(daemonProjects, [
      conversation("legacy-thread", UNFILED_PROJECT_ID),
    ]);

    assert.equal(tree.filter((node) => node.isUnfiled).length, 1);
    assert.deepEqual(
      tree[0]?.conversations.map((row) => row.id),
      ["legacy-thread"],
    );
  });

  test("nests conversations and groups legacy or orphaned rows under Unfiled", () => {
    const tree = buildProjectTree(projects, [
      conversation("alpha-thread", "project-a"),
      conversation("legacy-thread"),
      conversation("orphan-thread", "missing-project"),
    ]);

    assert.deepEqual(tree.map((node) => node.id), [
      "project-a",
      "project-b",
      UNFILED_PROJECT_ID,
    ]);
    assert.deepEqual(
      tree[0]?.conversations.map((row) => row.id),
      ["alpha-thread"],
    );
    assert.deepEqual(
      tree[2]?.conversations.map((row) => row.id),
      ["legacy-thread", "orphan-thread"],
    );
  });

  test("omits Unfiled when every conversation belongs to a known project", () => {
    const tree = buildProjectTree([
      {
        id: UNFILED_PROJECT_ID,
        name: "Unfiled",
        created_at: "1970-01-01T00:00:00Z",
        updated_at: "1970-01-01T00:00:00Z",
        roots: [],
      },
      ...projects,
    ], [
      conversation("alpha-thread", "project-a"),
    ]);
    assert.equal(tree.some((node) => node.isUnfiled), false);
  });
});

describe("UI store migration", () => {
  test("adds project state and preserves clamped sidebar widths", () => {
    assert.deepEqual(
      migrateUiState({
        sidebarWidth: 9999,
        artifactSidebarWidth: 12,
      }),
      {
        sidebarWidth: 480,
        artifactSidebarWidth: 280,
        expandedProjectIds: [],
        selectedProjectId: null,
      },
    );
  });

  test("repairs malformed project state and removes duplicate ids", () => {
    assert.deepEqual(
      migrateUiState({
        expandedProjectIds: ["a", 4, "a", "b"],
        selectedProjectId: 42,
      }),
      {
        expandedProjectIds: ["a", "b"],
        selectedProjectId: null,
      },
    );
  });
});
