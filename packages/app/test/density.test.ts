import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  deriveDensityTokens,
  migrateAppearanceDensityState,
  normalizeUiDensity,
} from "../src/styles/density";

describe("density tokens", () => {
  test("derives ordered spacing across every mode", () => {
    const compact = deriveDensityTokens("compact");
    const comfortable = deriveDensityTokens("comfortable");
    const spacious = deriveDensityTokens("spacious");

    assert.ok(compact.rowHeight < comfortable.rowHeight);
    assert.ok(comfortable.rowHeight < spacious.rowHeight);
    assert.ok(compact.chatGutter < comfortable.chatGutter);
    assert.ok(comfortable.settingsRowPaddingY < spacious.settingsRowPaddingY);
    assert.equal(comfortable.rowHeight, 28);
  });

  test("normalizes unknown values to comfortable", () => {
    assert.equal(normalizeUiDensity("compact"), "compact");
    assert.equal(normalizeUiDensity("dense"), "comfortable");
    assert.equal(normalizeUiDensity(null), "comfortable");
  });
});

describe("appearance density migration", () => {
  test("adds the default to older appearance state without dropping fields", () => {
    assert.deepEqual(
      migrateAppearanceDensityState({ themeMode: "dark", uiFontSize: 15 }),
      {
        themeMode: "dark",
        uiFontSize: 15,
        density: "comfortable",
      },
    );
  });

  test("repairs invalid persisted density", () => {
    assert.deepEqual(migrateAppearanceDensityState({ density: "tiny" }), {
      density: "comfortable",
    });
  });
});
