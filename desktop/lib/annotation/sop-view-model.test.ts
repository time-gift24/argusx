import { describe, expect, it } from "vitest";
import {
  buildSopFieldKey,
  buildSopSectionId,
  pickDefaultSopStep,
} from "@/lib/annotation/sop-view-model";

describe("sop view model", () => {
  it("builds field_key and section_id with sop_step_id", () => {
    expect(buildSopFieldKey(12, "operation")).toBe("sop.12.operation");
    expect(buildSopSectionId(12)).toBe("sop-step-12");
  });

  it("picks first available step in detect->handle->verification->rollback order", () => {
    const step = pickDefaultSopStep({
      detect: [],
      handle: [{ sop_step_id: 21, name: "处理A", version: 1 }],
      verification: [{ sop_step_id: 31, name: "验证A", version: 1 }],
      rollback: [],
    });

    expect(step?.sop_step_id).toBe(21);
  });
});
