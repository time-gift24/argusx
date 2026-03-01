import { describe, expect, it, vi } from "vitest";
import { loadRuleCatalog } from "@/lib/annotation/loaders";

describe("loadRuleCatalog", () => {
  it("returns fallback data when remote fails", async () => {
    const remote = vi.fn().mockRejectedValue(new Error("network"));
    const out = await loadRuleCatalog(remote);
    expect(out.source).toBe("fallback");
    expect(out.items.length).toBeGreaterThan(0);
  });
});
