"use client";

import * as React from "react";
import { useQuillSelectionAnchor } from "@/hooks/use-quill-selection-anchor";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import type { AnnotationLocation } from "@/lib/annotation/types";
import type Quill from "quill";

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
  const hostRef = React.useRef<HTMLDivElement>(null);
  const quillRef = React.useRef<Quill | null>(null);
  const textRef = React.useRef(text);
  textRef.current = text;

  const state = useAnnotationStore((store) => store.state);
  const dispatch = useAnnotationStore((store) => store.dispatch);

  const onSelectionChange = useQuillSelectionAnchor({
    delayMs: 300,
    onFire: (range) => {
      const start = range.index;
      const end = range.index + range.length;
      const selectedText = quillRef.current?.getText(start, range.length) ?? textRef.current.slice(start, end);

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
  const onSelectionChangeRef = React.useRef(onSelectionChange);
  onSelectionChangeRef.current = onSelectionChange;

  const isAnnotated = state.items.some((item) => item.location.field_key === fieldKey);

  React.useEffect(() => {
    let isCancelled = false;
    let cleanup: (() => void) | undefined;

    async function mountEditor() {
      if (!hostRef.current || quillRef.current) {
        return;
      }

      const quillModule = await import("quill");
      if (isCancelled || !hostRef.current) {
        return;
      }

      const QuillCtor = quillModule.default;
      const quill = new QuillCtor(hostRef.current, {
        theme: "bubble",
        readOnly: true,
        modules: {
          toolbar: false,
        },
      });

      quill.setText(textRef.current);

      const handleSelectionChange = (range: { index: number; length: number } | null) => {
        onSelectionChangeRef.current(range);
      };

      quill.on("selection-change", handleSelectionChange);
      quillRef.current = quill;

      cleanup = () => {
        quill.off("selection-change", handleSelectionChange);
        if (hostRef.current) {
          hostRef.current.innerHTML = "";
        }
        quillRef.current = null;
      };
    }

    void mountEditor();

    return () => {
      isCancelled = true;
      cleanup?.();
    };
  }, []);

  React.useEffect(() => {
    const quill = quillRef.current;
    if (!quill) {
      return;
    }
    const currentText = quill.getText();
    const nextText = text.endsWith("\n") ? text : `${text}\n`;
    if (currentText !== nextText) {
      quill.setText(text);
    }
  }, [text]);

  return (
    <div className={`space-y-1 rounded-md border p-2 ${isAnnotated ? "border-emerald-500" : ""}`}>
      <label className="text-sm font-medium">{label}</label>
      <div
        ref={hostRef}
        className="min-h-[70px] rounded-md border bg-background px-2 py-1 text-sm [&_.ql-editor]:min-h-[70px] [&_.ql-editor]:p-0"
      />
    </div>
  );
}
