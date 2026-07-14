import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  isSettingsSection,
  normalizeSettingsSection,
  sectionFromPathname,
  SETTINGS_NAV_GROUPS,
  SETTINGS_NAV_ITEMS,
  SETTINGS_SECTION_IDS,
} from "../src/lib/settings-navigation";
import { isConversationRouteId } from "../src/lib/route-path";
import {
  rankSettingsSearchEntries,
  settingsSearchEntryTarget,
} from "../src/lib/settings-search-index";

describe("settings navigation", () => {
  test("groups every valid route exactly once in taxonomy order", () => {
    assert.deepEqual(
      SETTINGS_NAV_GROUPS.flatMap((group) =>
        SETTINGS_NAV_ITEMS.filter((item) => item.group === group.id).map((item) => item.id),
      ),
      SETTINGS_SECTION_IDS,
    );
    assert.equal(new Set(SETTINGS_NAV_ITEMS.map((item) => item.id)).size, SETTINGS_SECTION_IDS.length);
  });

  test("validates slugs and defaults invalid routes to general", () => {
    for (const section of SETTINGS_SECTION_IDS) assert.equal(isSettingsSection(section), true);
    assert.equal(isSettingsSection("agents"), false);
    assert.equal(isSettingsSection("skills"), false);
    assert.equal(normalizeSettingsSection("providers"), "providers");
    assert.equal(normalizeSettingsSection("unknown"), "general");
  });

  test("derives the active section from the route pathname, not a stale default", () => {
    assert.equal(sectionFromPathname("/settings/providers"), "providers");
    assert.equal(sectionFromPathname("/settings/usage"), "usage");
    assert.equal(sectionFromPathname("/settings/connect?target=foo"), "connect");
    assert.equal(sectionFromPathname("/settings"), "general");
    assert.equal(sectionFromPathname("/"), "general");
    assert.equal(sectionFromPathname("/settings/unknown-section"), "general");
  });
});

describe("route slug guards", () => {
  test("accepts UUID conversation ids and rejects arbitrary path segments", () => {
    assert.equal(isConversationRouteId("019be9d2-44e1-7fb0-a361-a8b66b10c699"), true);
    assert.equal(isConversationRouteId("not-a-conversation"), false);
    assert.equal(isConversationRouteId(["019be9d2-44e1-7fb0-a361-a8b66b10c699"]), false);
  });
});

describe("settings search", () => {
  test("ranks exact titles ahead of keyword matches", () => {
    const results = rankSettingsSearchEntries("provider usage");
    assert.equal(results[0]?.section, "usage");
    assert.equal(results[0]?.title, "Provider usage");
  });

  test("finds provider auth guidance and derives stable row anchors", () => {
    const [result] = rankSettingsSearchEntries("sign in");
    assert.equal(result?.section, "providers");

    const [density] = rankSettingsSearchEntries("interface density");
    assert.equal(settingsSearchEntryTarget(density!), "setting-interface-density");
  });

  test("returns no results for blank or unrelated queries", () => {
    assert.deepEqual(rankSettingsSearchEntries(""), []);
    assert.deepEqual(rankSettingsSearchEntries("nonexistent-setting"), []);
  });
});
