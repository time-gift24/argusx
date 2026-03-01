import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QuillReviewField } from "@/components/features/annotation/quill-review-field";
import { initialAnnotationState } from "@/lib/annotation/reducer";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

const { mockQuillInstances } = vi.hoisted(() => ({
  mockQuillInstances: [] as MockQuillInstance[],
}));

type MockRange = { index: number; length: number };

type MockQuillInstance = {
  setText: (value: string) => void;
  getText: (index?: number, length?: number) => string;
  formatText: (
    index: number,
    length: number,
    format: { background?: string | false },
    source?: string,
  ) => void;
  on: (event: string, handler: (range: MockRange | null) => void) => void;
  off: (event: string, handler: (range: MockRange | null) => void) => void;
  emitSelection: (range: MockRange | null) => void;
  formatCalls: Array<{
    index: number;
    length: number;
    format: { background?: string | false };
    source?: string;
  }>;
};

vi.mock("quill", () => {
  class MockQuill implements MockQuillInstance {
    private text = "\n";

    private readonly listeners = new Map<string, Set<(range: MockRange | null) => void>>();
    readonly formatCalls: Array<{
      index: number;
      length: number;
      format: { background?: string | false };
      source?: string;
    }> = [];

    constructor(host: HTMLElement) {
      const container = document.createElement("div");
      container.className = "ql-container";
      const editor = document.createElement("div");
      editor.className = "ql-editor";
      container.appendChild(editor);
      host.appendChild(container);

      mockQuillInstances.push(this);
    }

    setText(value: string) {
      this.text = value.endsWith("\n") ? value : `${value}\n`;
    }

    getText(index?: number, length?: number) {
      if (index === undefined) {
        return this.text;
      }
      if (length === undefined) {
        return this.text.slice(index);
      }
      return this.text.slice(index, index + length);
    }

    formatText(
      index: number,
      length: number,
      format: { background?: string | false },
      source?: string,
    ) {
      this.formatCalls.push({ index, length, format, source });
    }

    on(event: string, handler: (range: MockRange | null) => void) {
      const existing = this.listeners.get(event) ?? new Set();
      existing.add(handler);
      this.listeners.set(event, existing);
    }

    off(event: string, handler: (range: MockRange | null) => void) {
      this.listeners.get(event)?.delete(handler);
    }

    emitSelection(range: MockRange | null) {
      for (const handler of this.listeners.get("selection-change") ?? []) {
        handler(range);
      }
    }
  }

  return {
    default: MockQuill,
  };
});

describe("QuillReviewField selection mapping", () => {
  beforeEach(() => {
    mockQuillInstances.length = 0;
    useAnnotationStore.setState((current) => ({
      ...current,
      state: initialAnnotationState,
      catalog: [],
      catalogSource: null,
    }));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("opens annotation with quill range offsets and selected text", async () => {
    render(
      <QuillReviewField
        sectionId="paragraph-1"
        fieldKey="paragraph.summary"
        label="段落摘要"
        text="ABCDEFGH"
      />,
    );

    await waitFor(() => {
      expect(mockQuillInstances.length).toBe(1);
    });

    vi.useFakeTimers();
    act(() => {
      mockQuillInstances[0].emitSelection({ index: 2, length: 3 });
      vi.advanceTimersByTime(300);
    });

    const state = useAnnotationStore.getState().state;
    expect(state.activeId).not.toBeNull();
    expect(state.items).toHaveLength(1);

    const current = state.items[0];
    expect(current.location.source_type).toBe("rich_text_selection");
    expect(current.location.section_id).toBe("paragraph-1");
    expect(current.location.field_key).toBe("paragraph.summary");
    expect(current.location.start_offset).toBe(2);
    expect(current.location.end_offset).toBe(5);
    expect(current.location.selected_text).toBe("CDE");
  });

  it("applies text highlights for all ranges and stronger color for active", async () => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: {
        items: [
          {
            id: "ann-a",
            location: {
              source_type: "rich_text_selection",
              panel: "paragraph_detail",
              section_id: "paragraph-1",
              field_key: "paragraph.summary",
              node_id: "paragraph.summary-node",
              start_offset: 1,
              end_offset: 3,
              selected_text: "BC",
            },
            ruleCode: null,
            payload: {},
            status: "submitted",
            updatedAt: 1,
          },
          {
            id: "ann-b",
            location: {
              source_type: "rich_text_selection",
              panel: "paragraph_detail",
              section_id: "paragraph-1",
              field_key: "paragraph.summary",
              node_id: "paragraph.summary-node",
              start_offset: 2,
              end_offset: 5,
              selected_text: "CDE",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 2,
          },
          {
            id: "ann-c",
            location: {
              source_type: "rich_text_selection",
              panel: "paragraph_detail",
              section_id: "paragraph-1",
              field_key: "paragraph.summary",
              node_id: "paragraph.summary-node",
              start_offset: 0,
              end_offset: 2,
              selected_text: "AB",
            },
            ruleCode: null,
            payload: {},
            status: "orphaned",
            updatedAt: 3,
          },
          {
            id: "ann-d",
            location: {
              source_type: "rich_text_selection",
              panel: "paragraph_detail",
              section_id: "paragraph-1",
              field_key: "paragraph.summary",
              node_id: "paragraph.summary-node",
              start_offset: 5,
              end_offset: 7,
              selected_text: "FG",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 4,
          },
        ],
        activeId: "ann-b",
      },
    }));

    render(
      <QuillReviewField
        sectionId="paragraph-1"
        fieldKey="paragraph.summary"
        label="段落摘要"
        text="ABCDEFGH"
      />,
    );

    await waitFor(() => {
      expect(mockQuillInstances.length).toBe(1);
      expect(mockQuillInstances[0].formatCalls.length).toBeGreaterThanOrEqual(3);
    });

    const [clearCall, baseCall, activeCall] = mockQuillInstances[0].formatCalls;
    expect(mockQuillInstances[0].formatCalls).toHaveLength(3);
    expect(clearCall).toEqual({
      index: 0,
      length: 8,
      format: { background: false },
      source: "silent",
    });
    expect(baseCall).toEqual({
      index: 1,
      length: 2,
      format: { background: "rgba(16, 185, 129, 0.24)" },
      source: "silent",
    });
    expect(activeCall).toEqual({
      index: 2,
      length: 3,
      format: { background: "rgba(16, 185, 129, 0.42)" },
      source: "silent",
    });
  });
});
