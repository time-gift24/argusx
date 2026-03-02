"use client";

import { mockReviewData } from "./mock-review-data";
import { createLocationFingerprint } from "@/lib/annotation/location";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import type { AnnotationLocation } from "@/lib/annotation/types";
import { toast } from "sonner";

const FIELDS = [
  { key: "case_title", label: "案件标题", value: mockReviewData.basicInfo.case_title },
  { key: "case_summary", label: "案件摘要", value: mockReviewData.basicInfo.case_summary },
  { key: "sop_id", label: "SOP ID", value: mockReviewData.sop?.sop_id ?? "" },
  { key: "sop_name", label: "SOP 名称", value: mockReviewData.sop?.name ?? "" },
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
        const location = buildPlainLocation(field.key, field.value);
        const locationFingerprint = createLocationFingerprint(location);
        const existing = state.items.find(
          (item) => createLocationFingerprint(item.location) === locationFingerprint,
        );
        const isSubmitted = existing?.status === "submitted";
        const isActiveDraft = existing?.id === state.activeId && existing.status === "draft";
        const fieldClass = isSubmitted
          ? "border-emerald-500/40 bg-emerald-500/10 dark:border-emerald-400/40 dark:bg-emerald-500/20"
          : isActiveDraft
            ? "border-border bg-muted"
            : "border-border/60 bg-background hover:bg-muted/50";

        return (
          <button
            key={field.key}
            type="button"
            data-testid={`annotatable-field-${field.key}`}
            className={`w-full rounded-md border p-3 text-left ${fieldClass}`}
            onClick={() => {
              dispatch({
                type: "OPEN",
                location,
              });

              if (!existing) {
                toast.info("新的标注事件", {
                  description: field.label,
                });
              }
            }}
          >
            <div className="text-sm font-medium">{field.label}</div>
            <div className="mt-1 text-sm text-muted-foreground">{field.value}</div>
          </button>
        );
      })}
    </div>
  );
}
