import { render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { QuillReviewField } from "@/components/features/annotation/quill-review-field";

describe("QuillReviewField", () => {
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
});

