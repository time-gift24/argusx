import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { QuillReviewField } from "@/components/features/annotation/quill-review-field";
import { initialAnnotationState } from "@/lib/annotation/reducer";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

describe("QuillReviewField", () => {
  beforeEach(() => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: initialAnnotationState,
      catalog: [],
      catalogSource: null,
    }));
  });

  it("renders using quill editor container instead of textarea", async () => {
    const { container } = render(
      <QuillReviewField
        sectionId="paragraph-1"
        fieldKey="paragraph.summary"
        label="段落摘要"
        text="这是一个用于测试的富文本字段。"
      />,
    );

    await waitFor(() => {
      expect(container.querySelector(".ql-container")).toBeInTheDocument();
    });

    expect(container.querySelector("textarea")).not.toBeInTheDocument();
  });

  it("does not use container border as annotation highlight", async () => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: {
        items: [
          {
            id: "ann-rich-1",
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
            updatedAt: 1,
          },
        ],
        activeId: "ann-rich-1",
      },
    }));

    const { container } = render(
      <QuillReviewField
        sectionId="paragraph-1"
        fieldKey="paragraph.summary"
        label="段落摘要"
        text="ABCDEFGH"
      />,
    );

    await waitFor(() => {
      expect(container.querySelector(".ql-container")).toBeInTheDocument();
    });

    const wrapper = container.querySelector(".space-y-1.rounded-md.border.p-2");
    expect(wrapper).toBeInTheDocument();
    expect(wrapper).not.toHaveClass("border-emerald-500");
  });
});
