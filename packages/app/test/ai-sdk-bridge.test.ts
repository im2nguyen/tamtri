import assert from "node:assert/strict";
import { describe, test } from "node:test";

import type { UiEvent } from "@tamtri/protocol";

import {
  contentBlockToParts,
  contentBlocksToParts,
  projectTranscriptToUIMessages,
} from "../src/lib/ai-sdk-bridge";
import type { ContentBlock, TranscriptMessage } from "../src/lib/transcript";
import {
  applyUiMessageChunks,
  createLiveUiProjection,
  uiMessageChunksFromUiEvent,
} from "../src/lib/ui-message-stream";

describe("contentBlockToParts", () => {
  test("maps text and thinking blocks", () => {
    assert.deepEqual(contentBlockToParts({ type: "text", text: "hello" }), [
      { type: "text", text: "hello", state: "done" },
    ]);
    assert.deepEqual(contentBlockToParts({ type: "thinking", text: "hmm" }), [
      { type: "reasoning", text: "hmm", state: "done" },
    ]);
  });

  test("maps tool call and artifact blocks", () => {
    const toolParts = contentBlockToParts({
      type: "tool_call",
      id: "call-1",
      name: "echo",
      input: { value: 1 },
    });
    assert.equal(toolParts.length, 1);
    assert.equal(toolParts[0]?.type, "tool-echo");

    const artifactParts = contentBlockToParts({
      type: "artifact",
      path: "attachments/report.html",
      mime_type: "text/html",
      size: 128,
      sha256: "abc",
      inline: "<h1>ok</h1>",
    });
    assert.equal(artifactParts[0]?.type, "data-tamtri-artifact");
  });

  test("merges tool call and result into one part", () => {
    const blocks: ContentBlock[] = [
      { type: "tool_call", id: "call-1", name: "echo", input: { x: 1 } },
      { type: "tool_result", call_id: "call-1", output: "ok" },
    ];
    const parts = contentBlocksToParts(blocks);
    assert.equal(parts.length, 1);
    const tool = parts[0];
    assert.ok(tool && tool.type === "tool-echo");
    if (tool.type === "tool-echo") {
      assert.equal(tool.state, "output-available");
      assert.equal(tool.output, "ok");
    }
  });
});

describe("projectTranscriptToUIMessages", () => {
  test("projects assistant transcript with mixed blocks", () => {
    const messages: TranscriptMessage[] = [
      {
        id: "u1",
        role: "user",
        content: [{ type: "text", text: "Hi" }],
        created_at: "2026-01-01T00:00:00Z",
      },
      {
        id: "a1",
        role: "assistant",
        harness_id: "acp:test",
        content: [
          { type: "thinking", text: "plan" },
          { type: "text", text: "Done." },
        ],
        created_at: "2026-01-01T00:00:01Z",
      },
    ];

    const projected = projectTranscriptToUIMessages(messages);
    assert.equal(projected.length, 2);
    assert.equal(projected[1]?.role, "assistant");
    assert.equal(projected[1]?.metadata?.harness_id, "acp:test");
    assert.equal(projected[1]?.parts.length, 2);
    assert.equal(projected[1]?.parts[0]?.type, "reasoning");
    assert.equal(projected[1]?.parts[1]?.type, "text");
  });
});

describe("uiMessageChunksFromUiEvent", () => {
  test("streams text incrementally", () => {
    const event: UiEvent = {
      conversation_id: "c1",
      kind: "text_delta",
      payload_json: JSON.stringify({ type: "text_delta", text: "Hel" }),
    };
    const chunks = uiMessageChunksFromUiEvent(event);
    assert.ok(chunks);
    assert.equal(chunks[0]?.type, "text-delta");

    let state = createLiveUiProjection();
    state = applyUiMessageChunks(state, chunks);
    state = applyUiMessageChunks(
      state,
      uiMessageChunksFromUiEvent({
        ...event,
        payload_json: JSON.stringify({ type: "text_delta", text: "lo" }),
      })!,
    );

    const textPart = state.message.parts.find((part) => part.type === "text");
    assert.ok(textPart && textPart.type === "text");
    assert.equal(textPart.text, "Hello");
    assert.equal(textPart.state, "streaming");
  });

  test("maps tool lifecycle chunks", () => {
    const started: UiEvent = {
      conversation_id: "c1",
      kind: "tool_call_started",
      payload_json: JSON.stringify({
        type: "tool_call_started",
        id: "t1",
        name: "read",
        input: { path: "/tmp/x" },
      }),
    };
    const completed: UiEvent = {
      conversation_id: "c1",
      kind: "tool_call_progress",
      payload_json: JSON.stringify({
        type: "tool_call_progress",
        id: "t1",
        status: "completed",
        content: [{ type: "text", text: "file contents" }],
      }),
    };

    let state = createLiveUiProjection();
    state = applyUiMessageChunks(state, uiMessageChunksFromUiEvent(started)!);
    state = applyUiMessageChunks(state, uiMessageChunksFromUiEvent(completed)!);

    const toolPart = state.message.parts.find((part) => part.type === "tool-read");
    assert.ok(toolPart && toolPart.type === "tool-read");
    assert.equal(toolPart.state, "output-available");
    assert.equal(toolPart.output, "file contents");
  });
});
