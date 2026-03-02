import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ParagraphPanel } from "@/components/features/annotation/paragraph-panel";

const { detailCalls } = vi.hoisted(() => ({
  detailCalls: [] as Array<{ stepId: number; stepName: string }>,
}));

vi.mock("@/components/features/annotation/sop-tree-nav", () => ({
  SopTreeNav: (props: {
    groups: {
      detect: Array<{ sop_step_id: number; name: string; version: number }>;
      handle: Array<{ sop_step_id: number; name: string; version: number }>;
      verification: Array<{ sop_step_id: number; name: string; version: number }>;
      rollback: Array<{ sop_step_id: number; name: string; version: number }>;
    };
    activeStepId: number | null;
    onSelect: (stepId: number) => void;
  }) => (
    <div data-testid="mock-sop-tree-nav">
      <p>detect:{props.groups.detect.length}</p>
      <button type="button" onClick={() => props.onSelect(21)}>
        select-21
      </button>
      <p>active:{props.activeStepId ?? "none"}</p>
    </div>
  ),
}));

vi.mock("@/components/features/annotation/sop-step-detail", () => ({
  SopStepDetail: (props: {
    stepId: number;
    stepName: string;
    stepDetail: Record<string, string>;
  }) => {
    detailCalls.push({ stepId: props.stepId, stepName: props.stepName });
    return <div data-testid="mock-sop-step-detail">{`detail-${props.stepId}-${props.stepName}`}</div>;
  },
}));

vi.mock("@/components/features/annotation/mock-review-data", () => ({
  mockReviewData: {
    docId: "doc-sop",
    basicInfo: {
      case_title: "某行政处罚案件",
      case_summary: "这是用于标注流程联调的示例摘要。",
    },
    sop: {
      sop_id: "sop-001",
      name: "SOP 样例",
      detect: [{ sop_step_id: 11, name: "检测A", version: 1 }],
      handle: [{ sop_step_id: 21, name: "处理A", version: 1 }],
      verification: [{ sop_step_id: 31, name: "验证A", version: 1 }],
      rollback: [{ sop_step_id: 41, name: "回退A", version: 1 }],
      step_details: {
        11: {
          id: 11,
          name: "检测A",
          operation: "<p>op</p>",
          verification: "<p>v</p>",
          impact_analysis: "<p>i</p>",
          rollback: "<p>r</p>",
        },
        21: {
          id: 21,
          name: "处理A",
          operation: "<p>op</p>",
          verification: "<p>v</p>",
          impact_analysis: "<p>i</p>",
          rollback: "<p>r</p>",
        },
      },
    },
  },
}));

describe("ParagraphPanel", () => {
  it("renders sop tree and defaults to first available step detail", () => {
    detailCalls.length = 0;
    render(<ParagraphPanel />);

    expect(screen.getByTestId("mock-sop-tree-nav")).toBeInTheDocument();
    expect(screen.getByTestId("mock-sop-step-detail")).toHaveTextContent("detail-11-检测A");
    expect(detailCalls.at(-1)).toEqual({ stepId: 11, stepName: "检测A" });
  });

  it("switches detail when selecting another step from tree", async () => {
    const user = userEvent.setup();
    detailCalls.length = 0;
    render(<ParagraphPanel />);

    await user.click(screen.getByRole("button", { name: "select-21" }));

    expect(screen.getByTestId("mock-sop-step-detail")).toHaveTextContent("detail-21-处理A");
    expect(detailCalls.at(-1)).toEqual({ stepId: 21, stepName: "处理A" });
  });
});
