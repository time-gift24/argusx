import { describe, expect, it } from "vitest";
import { annotationReducer, initialAnnotationState } from "@/lib/annotation/reducer";

const location = {
  source_type: "plain_field" as const,
  panel: "basic_info" as const,
  section_id: "base",
  field_key: "case_title",
  node_id: "case_title",
  start_offset: null,
  end_offset: null,
  selected_text: "",
};

describe("annotationReducer", () => {
  it("reuses existing annotation at same location", () => {
    const first = annotationReducer(initialAnnotationState, { type: "OPEN", location });
    const second = annotationReducer(first, { type: "OPEN", location });
    expect(second.items.length).toBe(1);
    expect(second.activeId).toBe(first.items[0].id);
  });

  it("autosaves previous draft when switching location", () => {
    const first = annotationReducer(initialAnnotationState, { type: "OPEN", location });
    const next = annotationReducer(first, {
      type: "OPEN",
      location: { ...location, field_key: "case_summary", node_id: "case_summary" },
    });
    const prev = next.items.find((i) => i.location.field_key === "case_title");
    expect(prev?.status).toBe("draft");
  });
});
