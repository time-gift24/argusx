import { describe, expect, it } from "vitest";
import { resolveRuleCatalog } from "@/lib/annotation/rule-catalog";
import { fallbackRules } from "@/lib/annotation/rules-fallback";

describe("resolveRuleCatalog", () => {
  it("returns remote rules on success", async () => {
    const remote = [{ code: "R1", label: "违规1", description: "d", version: 1, schema: [] }];
    const data = await resolveRuleCatalog(async () => remote, fallbackRules);
    expect(data.source).toBe("remote");
    expect(data.items).toEqual(remote);
  });

  it("falls back when remote throws", async () => {
    const data = await resolveRuleCatalog(async () => {
      throw new Error("network");
    }, fallbackRules);
    expect(data.source).toBe("fallback");
    expect(data.items).toEqual(fallbackRules);
  });
});
