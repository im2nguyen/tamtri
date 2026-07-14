import assert from "node:assert/strict";
import { before, describe, test } from "node:test";
import { parseHTML } from "linkedom";

import { collectBlockedHrefs, prepareArtifactHtml } from "../src/components/transcript/sandboxed-html-prep.web";

before(() => {
  class LinkedomDOMParser {
    parseFromString(html: string, type: string) {
      if (type !== "text/html") {
        throw new Error(`Unsupported MIME type: ${type}`);
      }
      return parseHTML(html).document;
    }
  }

  Object.defineProperty(globalThis, "DOMParser", {
    configurable: true,
    writable: true,
    value: LinkedomDOMParser,
  });
});

describe("prepareArtifactHtml", () => {
  test("removes script tags from output HTML", () => {
    const prepared = prepareArtifactHtml("<html><body><script>alert(1)</script><p>hi</p></body></html>");
    assert.doesNotMatch(prepared, /<script/i);
    assert.match(prepared, /<p>hi<\/p>/);
  });

  test("removes nested iframe tags", () => {
    const prepared = prepareArtifactHtml('<html><body><iframe src="https://evil"></iframe></body></html>');
    assert.doesNotMatch(prepared, /<iframe/i);
  });

  test("removes attacker CSP meta and keeps tamtri CSP", () => {
    const prepared = prepareArtifactHtml(
      '<html><head><meta http-equiv="Content-Security-Policy" content="connect-src *"></head><body></body></html>',
    );
    assert.doesNotMatch(prepared, /connect-src \*/i);
    assert.match(prepared, /connect-src 'none'/i);
    assert.match(prepared, /http-equiv="Content-Security-Policy"/i);
  });

  test("marks external anchors as blocked and preserves the original URL", () => {
    const prepared = prepareArtifactHtml(
      '<html><body><a href="https://example.com">link</a></body></html>',
    );
    assert.match(prepared, /data-tamtri-blocked-href="https:\/\/example\.com"/);
    assert.match(prepared, /href="#"/);
  });
});

describe("collectBlockedHrefs", () => {
  test("returns unique blocked URLs", () => {
    assert.deepEqual(
      collectBlockedHrefs('<html><body><a href="https://example.com">link</a></body></html>'),
      ["https://example.com"],
    );
  });
});
