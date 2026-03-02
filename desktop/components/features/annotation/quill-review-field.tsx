"use client";

import * as React from "react";
import { useQuillSelectionAnchor } from "@/hooks/use-quill-selection-anchor";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import type { AnnotationLocation } from "@/lib/annotation/types";
import type Quill from "quill";
import { toast } from "sonner";

type QuillReviewFieldProps = {
  sectionId: string;
  fieldKey: string;
  label: string;
  text: string;
  nodeId?: string;
};

type HighlightRange = {
  id: string;
  start: number;
  length: number;
  isActive: boolean;
};

const BASE_HIGHLIGHT_COLOR = "rgba(16, 185, 129, 0.24)";
const ACTIVE_HIGHLIGHT_COLOR = "rgba(16, 185, 129, 0.42)";

function normalizeQuillInputText(value: string): string {
  if (!value.includes("<")) {
    return value;
  }

  if (typeof document === "undefined") {
    return value.replace(/<[^>]+>/g, "");
  }

  const container = document.createElement("div");
  container.innerHTML = value;
  return container.textContent ?? "";
}

function buildHighlightRanges({
  items,
  activeId,
  fieldKey,
  textLength,
}: {
  items: ReturnType<typeof useAnnotationStore.getState>["state"]["items"];
  activeId: string | null;
  fieldKey: string;
  textLength: number;
}): HighlightRange[] {
  return items
    .filter(
      (item) =>
        item.status === "submitted" ||
        (item.status === "draft" && item.id === activeId),
    )
    .filter((item) => item.location.source_type === "rich_text_selection")
    .filter((item) => item.location.field_key === fieldKey)
    .map((item) => {
      const rawStart = item.location.start_offset ?? 0;
      const start = Math.max(0, Math.min(rawStart, textLength));
      const rawEnd = item.location.end_offset ?? start;
      const end = Math.max(start, Math.min(rawEnd, textLength));

      return {
        id: item.id,
        start,
        length: end - start,
        isActive: item.id === activeId,
      };
    })
    .filter((range) => range.length > 0);
}

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
  const [isEditorReady, setIsEditorReady] = React.useState(false);
  const normalizedText = React.useMemo(() => normalizeQuillInputText(text), [text]);
  const textRef = React.useRef(normalizedText);
  textRef.current = normalizedText;

  const state = useAnnotationStore((store) => store.state);
  const dispatch = useAnnotationStore((store) => store.dispatch);
  const highlightRanges = React.useMemo(
    () =>
      buildHighlightRanges({
        items: state.items,
        activeId: state.activeId,
        fieldKey,
        textLength: normalizedText.length,
      }),
    [fieldKey, normalizedText.length, state.activeId, state.items],
  );

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

      toast.info("新的标注事件", {
        description: `${label} [${start}, ${end})`,
      });
    },
  });
  const onSelectionChangeRef = React.useRef(onSelectionChange);
  onSelectionChangeRef.current = onSelectionChange;

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
      setIsEditorReady(true);

      cleanup = () => {
        quill.off("selection-change", handleSelectionChange);
        if (hostRef.current) {
          hostRef.current.innerHTML = "";
        }
        quillRef.current = null;
        setIsEditorReady(false);
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
    const nextText = normalizedText.endsWith("\n")
      ? normalizedText
      : `${normalizedText}\n`;
    if (currentText !== nextText) {
      quill.setText(normalizedText);
    }
  }, [normalizedText]);

  React.useEffect(() => {
    const quill = quillRef.current;
    if (!quill || !isEditorReady) {
      return;
    }

    quill.formatText(0, normalizedText.length, { background: false }, "silent");

    for (const range of highlightRanges.filter((item) => !item.isActive)) {
      quill.formatText(
        range.start,
        range.length,
        { background: BASE_HIGHLIGHT_COLOR },
        "silent",
      );
    }

    for (const range of highlightRanges.filter((item) => item.isActive)) {
      quill.formatText(
        range.start,
        range.length,
        { background: ACTIVE_HIGHLIGHT_COLOR },
        "silent",
      );
    }
  }, [highlightRanges, isEditorReady, normalizedText.length]);

  return (
    <div className="space-y-1 rounded-md border p-2">
      <label className="text-sm font-medium">{label}</label>
      <div
        ref={hostRef}
        className="min-h-[70px] rounded-md border bg-background px-2 py-1 text-sm [&_.ql-editor]:min-h-[70px] [&_.ql-editor]:p-0"
      />
    </div>
  );
}
