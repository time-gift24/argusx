import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SopTreeNav } from "@/components/features/annotation/sop-tree-nav";
import type { SopGroups } from "@/lib/annotation/sop-view-model";

describe("SopTreeNav", () => {
  const groups: SopGroups = {
    detect: [{ sop_step_id: 11, name: "检测A", version: 1 }],
    handle: [{ sop_step_id: 21, name: "处理A", version: 1 }],
    verification: [{ sop_step_id: 31, name: "验证A", version: 1 }],
    rollback: [{ sop_step_id: 41, name: "回退A", version: 1 }],
  };

  it("renders 4 fixed groups and all are expanded by default", () => {
    render(
      <SopTreeNav
        groups={groups}
        activeStepId={11}
        onSelect={() => {}}
      />,
    );

    expect(screen.getByText("01操作检测")).toBeInTheDocument();
    expect(screen.getByText("02操作处理")).toBeInTheDocument();
    expect(screen.getByText("03操作验证")).toBeInTheDocument();
    expect(screen.getByText("04操作回退")).toBeInTheDocument();

    expect(screen.getByRole("button", { name: "检测A" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "处理A" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "验证A" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "回退A" })).toBeInTheDocument();
  });

  it("calls onSelect with sop_step_id when step is clicked", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();

    render(
      <SopTreeNav
        groups={groups}
        activeStepId={11}
        onSelect={onSelect}
      />,
    );

    await user.click(screen.getByRole("button", { name: "处理A" }));

    expect(onSelect).toHaveBeenCalledWith(21);
  });
});
