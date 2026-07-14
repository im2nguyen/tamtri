import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { buildConfigTriggerLabel } from "../src/lib/composer-config-label";

describe("composer config menu", () => {
  test("buildConfigTriggerLabel joins harness and model with a middle dot", () => {
    assert.equal(buildConfigTriggerLabel("Hermes", "default"), "Hermes · default");
  });

  test("buildConfigTriggerLabel appends active mode when present", () => {
    assert.equal(buildConfigTriggerLabel("Claude Code", "opus", "plan"), "Claude Code · opus · Plan");
  });
});
