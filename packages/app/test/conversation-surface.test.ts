import assert from "node:assert/strict";
import { describe, test } from "node:test";
import type { TamtriUIMessage } from "@tamtri/protocol";

import {
  deriveRightDockState,
  isNearTranscriptBottom,
  summarizeToolActivity,
} from "../src/lib/conversation-surface";

describe("conversation activity summaries", () => {
  test("turns tool payloads into compact verb and object labels", () => {
    assert.deepEqual(
      summarizeToolActivity({
        toolName: "read_file",
        toolInput: { path: "/reports/q2.csv" },
        state: "output-available",
      }),
      {
        verb: "Read",
        object: "/reports/q2.csv",
        status: "completed",
        label: "Read /reports/q2.csv",
      },
    );
  });

  test("marks streaming and failed receipts without exposing raw payloads", () => {
    assert.equal(
      summarizeToolActivity({
        toolName: "search_records",
        toolInput: { query: "late renewals" },
        state: "input-streaming",
      }).status,
      "running",
    );
    assert.equal(
      summarizeToolActivity({
        toolName: "fetch",
        toolInput: { url: "https://example.test" },
        errorText: "offline",
      }).status,
      "failed",
    );
  });
});

describe("right dock state", () => {
  test("adds app and task tabs only when transcript data exists", () => {
    const messages: TamtriUIMessage[] = [
      {
        id: "assistant-1",
        role: "assistant",
        parts: [
          {
            type: "data-tamtri-app-resource",
            data: { uri: "ui://sales", template_ref: "sales", state: {} },
          },
          {
            type: "data-tamtri-task",
            data: { task_id: "task-1", status: "running", title: "Build report" },
          },
          {
            type: "data-tamtri-task",
            data: { task_id: "task-1", status: "completed", title: "Build report" },
          },
        ],
      },
    ];

    const state = deriveRightDockState(messages, 2);
    assert.deepEqual(
      state.tabs.map((tab) => [tab.id, tab.count]),
      [
        ["artifacts", 2],
        ["apps", 1],
        ["tasks", 1],
      ],
    );
    assert.equal(state.tasks[0]?.status, "completed");
  });

  test("keeps an empty dock closed to irrelevant tabs", () => {
    assert.deepEqual(deriveRightDockState([], 0).tabs, []);
  });
});

describe("transcript auto-follow", () => {
  test("follows within the bottom tolerance", () => {
    assert.equal(
      isNearTranscriptBottom({ contentHeight: 1200, viewportHeight: 600, offsetY: 540 }),
      true,
    );
  });

  test("stops following after an intentional scroll away", () => {
    assert.equal(
      isNearTranscriptBottom({ contentHeight: 1200, viewportHeight: 600, offsetY: 300 }),
      false,
    );
  });
});
