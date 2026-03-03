import { describe, expect, it } from "vitest";
import { shouldHighlightFence } from "./highlight-policy";

describe("shouldHighlightFence", () => {
  it("returns true for fenced + allowlisted language", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "rust" })).toBe(true);
  });

  it("returns false when language is empty", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "" })).toBe(false);
  });

  it("returns false for text and unknown languages", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "text" })).toBe(false);
    expect(shouldHighlightFence({ isFenced: true, language: "foo-lang" })).toBe(false);
  });

  it("returns false when not fenced", () => {
    expect(shouldHighlightFence({ isFenced: false, language: "rust" })).toBe(false);
  });
});
