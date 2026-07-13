import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  invalidateConversationList,
  subscribeConversationListInvalidation,
} from "../src/hooks/conversation-list-invalidation";

describe("conversation list invalidation bus", () => {
  test("notifies subscribed listeners", () => {
    let calls = 0;
    const unsubscribe = subscribeConversationListInvalidation(() => {
      calls += 1;
    });

    invalidateConversationList();
    assert.equal(calls, 1);

    unsubscribe();
    invalidateConversationList();
    assert.equal(calls, 1);
  });
});
