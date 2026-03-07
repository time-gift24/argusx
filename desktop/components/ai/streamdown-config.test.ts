import { describe, expect, it } from "vitest";

import { sharedStreamdownShikiTheme } from "@/components/ai/streamdown-config";

describe("sharedStreamdownShikiTheme", () => {
  it("uses a high-contrast dark code theme while keeping the current light theme", () => {
    expect(sharedStreamdownShikiTheme).toEqual([
      "github-light",
      "github-dark-high-contrast",
    ]);
  });
});
