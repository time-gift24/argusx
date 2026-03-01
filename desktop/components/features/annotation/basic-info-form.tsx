"use client";

import { mockReviewData } from "./mock-review-data";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import type { AnnotationLocation } from "@/lib/annotation/types";

const FIELDS = [
  { key: "case_title", label: "案件标题", value: mockReviewData.basicInfo.case_title },
  { key: "case_summary", label: "案件摘要", value: mockReviewData.basicInfo.case_summary },
];

function buildPlainLocation(fieldKey: string, selectedText: string): AnnotationLocation {
  return {
    source_type: "plain_field",
    panel: "basic_info",
    section_id: "base",
    field_key: fieldKey,
    node_id: fieldKey,
    start_offset: null,
    end_offset: null,
    selected_text: selectedText,
  };
}

export function BasicInfoForm() {
  const state = useAnnotationStore((store) => store.state);
  const dispatch = useAnnotationStore((store) => store.dispatch);

  return (
    <div className="space-y-3">
      {FIELDS.map((field) => {
        const isAnnotated = state.items.some((item) => item.location.field_key === field.key);

        return (
          <button
            key={field.key}
            type="button"
            data-testid={`annotatable-field-${field.key}`}
            className={`w-full rounded-md border p-3 text-left ${
              isAnnotated ? "border-emerald-500 bg-emerald-50" : "bg-background"
            }`}
            onClick={() =>
              dispatch({
                type: "OPEN",
                location: buildPlainLocation(field.key, field.value),
              })
            }
          >
            <div className="text-sm font-medium">{field.label}</div>
            <div className="mt-1 text-sm text-muted-foreground">{field.value}</div>
          </button>
        );
      })}
    </div>
  );
}
