"use client";

import * as React from "react";
import { useQuillSelectionAnchor } from "@/hooks/use-quill-selection-anchor";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import type { AnnotationLocation } from "@/lib/annotation/types";

type QuillReviewFieldProps = {
  sectionId: string;
  fieldKey: string;
  label: string;
  text: string;
  nodeId?: string;
};

function buildRichLocation(input: {
  sectionId: string;
  fieldKey: string;
  nodeId: string;
  start: number;
  end: number;
  selectedText: string;
}): AnnotationLocation {
  return {
    source_type: "rich_text_selection",
    panel: "paragraph_detail",
    section_id: input.sectionId,
    field_key: input.fieldKey,
    node_id: input.nodeId,
    start_offset: input.start,
    end_offset: input.end,
    selected_text: input.selectedText,
  };
}

export function QuillReviewField({
  sectionId,
  fieldKey,
  label,
  text,
  nodeId,
}: QuillReviewFieldProps) {
  const areaRef = React.useRef<HTMLTextAreaElement>(null);
  const state = useAnnotationStore((store) => store.state);
  const dispatch = useAnnotationStore((store) => store.dispatch);

  const onSelectionChange = useQuillSelectionAnchor({
    delayMs: 300,
    onFire: (range) => {
      const start = range.index;
      const end = range.index + range.length;
      const selectedText = text.slice(start, end);

      dispatch({
        type: "OPEN",
        location: buildRichLocation({
          sectionId,
          fieldKey,
          nodeId: nodeId ?? `${fieldKey}-node`,
          start,
          end,
          selectedText,
        }),
      });
    },
  });

  const isAnnotated = state.items.some((item) => item.location.field_key === fieldKey);

  function handleSelectionStop() {
    const area = areaRef.current;
    if (!area) return;

    const start = area.selectionStart ?? 0;
    const end = area.selectionEnd ?? 0;
    onSelectionChange({ index: start, length: Math.max(0, end - start) });
  }

  return (
    <div className={`space-y-1 rounded-md border p-2 ${isAnnotated ? "border-emerald-500" : ""}`}>
      <label className="text-sm font-medium">{label}</label>
      <textarea
        ref={areaRef}
        readOnly
        value={text}
        onMouseUp={handleSelectionStop}
        onKeyUp={handleSelectionStop}
        className="min-h-[70px] w-full rounded-md border bg-background px-2 py-1 text-sm"
      />
    </div>
  );
}
