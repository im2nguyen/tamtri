import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { method } from "@tamtri/protocol";

describe("project mutation RPC names", () => {
  test("uses documented move and root remove methods", () => {
    assert.equal(method.CONVERSATION_MOVE_PROJECT, "conversation.move_project");
    assert.equal(method.PROJECT_ROOT_REMOVE, "project.root_remove");
  });
});
