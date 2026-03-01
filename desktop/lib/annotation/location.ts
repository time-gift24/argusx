import type { AnnotationLocation } from "./types";

export function createLocationFingerprint(location: AnnotationLocation): string {
  return [
    location.panel,
    location.section_id,
    location.field_key,
    location.node_id,
    location.start_offset ?? "null",
    location.end_offset ?? "null",
  ].join("|");
}

export function isRichTextLocation(location: AnnotationLocation): boolean {
  return location.source_type === "rich_text_selection";
}
