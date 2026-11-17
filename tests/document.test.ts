import { describe, expect, test } from "bun:test";
import { createDocument, parseDocument, serializeDocument } from "../src/core/document";
import { createRectangle } from "../src/core/geometry";
import { setKeyframe } from "../src/core/timeline";

describe("project document", () => {
  test("round-trips a document with keyframes", () => {
    const rectangle = createRectangle({ x: 12, y: 14 }, { x: 120, y: 90 });
    let document = createDocument(24);
    document.rectangle = rectangle;
    document.timeline = setKeyframe(document.timeline, 0, rectangle);
    const restored = parseDocument(serializeDocument(document));
    expect(restored).toEqual(document);
  });
  test("rejects unknown project versions", () => {
    expect(() => parseDocument(JSON.stringify({ version: 99, timeline: { fps: 24, keyframes: [] }, rectangle: null })))
      .toThrow("Unsupported project version");
  });
});
