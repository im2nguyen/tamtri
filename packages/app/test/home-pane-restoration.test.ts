import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  isValidComposeProjectId,
  resolveHomeRestoration,
} from "../src/lib/home-restoration";
import { UNFILED_PROJECT_ID } from "../src/lib/project-tree";

const validProjectIds = new Set(["project-a", UNFILED_PROJECT_ID]);

describe("resolveHomeRestoration", () => {
  test("stays on home when draft project id is valid, including Unfiled", () => {
    assert.deepEqual(
      resolveHomeRestoration({
        draftProjectId: UNFILED_PROJECT_ID,
        validProjectIds,
        conversations: [{ id: "latest-thread", project_id: "project-a" }],
      }),
      { action: "stay", projectId: UNFILED_PROJECT_ID },
    );
  });

  test("redirects to the latest conversation when no draft is active", () => {
    assert.deepEqual(
      resolveHomeRestoration({
        draftProjectId: null,
        validProjectIds,
        conversations: [{ id: "latest-thread", project_id: "project-a" }],
      }),
      {
        action: "redirect",
        conversationId: "latest-thread",
        projectId: "project-a",
      },
    );
  });

  test("stays on home when there are no conversations to restore", () => {
    assert.deepEqual(
      resolveHomeRestoration({
        draftProjectId: null,
        validProjectIds,
        conversations: [],
      }),
      { action: "stay" },
    );
  });
});

describe("isValidComposeProjectId", () => {
  test("accepts Unfiled as a valid compose target", () => {
    assert.equal(
      isValidComposeProjectId(
        [{ id: UNFILED_PROJECT_ID, name: "Unfiled" }],
        UNFILED_PROJECT_ID,
      ),
      true,
    );
  });
});
