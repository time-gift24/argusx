import { describe, expect, it } from "vitest";
import { createLocationFingerprint, isRichTextLocation } from "@/lib/annotation/location";
import type { AnnotationLocation } from "@/lib/annotation/types";

const rich: AnnotationLocation = {
  source_type: "rich_text_selection",
  panel: "paragraph_detail",
  section_id: "sec-1",
  field_key: "paragraph.summary",
  node_id: "node-22",
  start_offset: 12,
  end_offset: 19,
  selected_text: "违规片段",
};

describe("location helpers", () => {
  it("creates stable fingerprints", () => {
    expect(createLocationFingerprint(rich)).toBe(
      "paragraph_detail|sec-1|paragraph.summary|node-22|12|19",
    );
  });

  it("detects rich text location", () => {
    expect(isRichTextLocation(rich)).toBe(true);
  });
});
