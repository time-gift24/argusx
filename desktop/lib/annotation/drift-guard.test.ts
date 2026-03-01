import Delta from "quill-delta";
import { describe, expect, it } from "vitest";
import { reanchorByDelta } from "@/lib/annotation/drift-guard";

describe("reanchorByDelta", () => {
  it("shifts offsets after insertion before range", () => {
    const delta = new Delta().retain(3).insert("XYZ");
    const out = reanchorByDelta({ start: 10, end: 15 }, delta);
    expect(out).toEqual({ start: 13, end: 18, status: "ok" });
  });

  it("marks orphaned when selected text mismatches", () => {
    const delta = new Delta().retain(1).insert("A");
    const out = reanchorByDelta(
      { start: 2, end: 5, selectedText: "foo", currentTextAtRange: "bar" },
      delta,
    );
    expect(out.status).toBe("orphaned");
  });
});
