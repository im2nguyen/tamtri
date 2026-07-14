import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, test } from "node:test";
import { fileURLToPath } from "node:url";

describe("conversation list provider wiring", () => {
  test("re-exports the shared provider hook from use-conversations", () => {
    const dir = dirname(fileURLToPath(import.meta.url));
    const hookSource = readFileSync(join(dir, "../src/hooks/use-conversations.ts"), "utf8");
    assert.match(hookSource, /conversation-list-provider/);
  });

  test("mounts the provider once in the app layout", () => {
    const dir = dirname(fileURLToPath(import.meta.url));
    const layoutSource = readFileSync(join(dir, "../src/app/_layout.tsx"), "utf8");
    assert.match(layoutSource, /ConversationListProvider/);
  });
});
