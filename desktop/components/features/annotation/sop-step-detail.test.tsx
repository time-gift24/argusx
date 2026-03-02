import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SopStepDetail } from "@/components/features/annotation/sop-step-detail";

const { quillCalls } = vi.hoisted(() => ({
  quillCalls: [] as Array<{
    sectionId: string;
    fieldKey: string;
    label: string;
    text: string;
  }>,
}));

vi.mock("@/components/features/annotation/quill-review-field", () => ({
  QuillReviewField: (props: {
    sectionId: string;
    fieldKey: string;
    label: string;
    text: string;
  }) => {
    quillCalls.push(props);
    return <div data-testid={`quill-${props.fieldKey}`}>{props.label}</div>;
  },
}));

describe("SopStepDetail", () => {
  beforeEach(() => {
    quillCalls.length = 0;
  });

  it("renders 4 fixed sections and maps sop field keys", () => {
    render(
      <SopStepDetail
        stepId={12}
        stepName="处理A"
        stepDetail={{
          operation: "<p>执行步骤</p>",
          verification: "<p>验证步骤</p>",
          impact_analysis: "<p>影响分析</p>",
          rollback: "<p>回退步骤</p>",
        }}
      />,
    );

    expect(screen.getByText("处理A")).toBeInTheDocument();
    expect(screen.getAllByText("操作步骤").length).toBeGreaterThan(0);
    expect(screen.getAllByText("操作验证").length).toBeGreaterThan(0);
    expect(screen.getAllByText("影响分析").length).toBeGreaterThan(0);
    expect(screen.getAllByText("操作回退").length).toBeGreaterThan(0);

    expect(quillCalls).toHaveLength(4);
    expect(quillCalls.map((call) => call.fieldKey)).toEqual([
      "sop.12.operation",
      "sop.12.verification",
      "sop.12.impact_analysis",
      "sop.12.rollback",
    ]);
    expect(new Set(quillCalls.map((call) => call.sectionId))).toEqual(
      new Set(["sop-step-12"]),
    );
  });

  it("keeps all sections and fills empty text when detail field is missing", () => {
    render(
      <SopStepDetail
        stepId={9}
        stepName="检测A"
        stepDetail={{
          operation: "<p>执行步骤</p>",
        }}
      />,
    );

    expect(quillCalls).toHaveLength(4);
    expect(quillCalls.find((call) => call.fieldKey === "sop.9.verification")?.text).toBe("");
    expect(quillCalls.find((call) => call.fieldKey === "sop.9.impact_analysis")?.text).toBe("");
    expect(quillCalls.find((call) => call.fieldKey === "sop.9.rollback")?.text).toBe("");
  });
});
