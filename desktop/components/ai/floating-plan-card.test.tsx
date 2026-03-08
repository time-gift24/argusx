import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { FloatingPlanCard } from "@/components/ai/floating-plan-card";

describe("FloatingPlanCard", () => {
  it("wraps PlanQueue in a floating container", () => {
    const { container } = render(
      <FloatingPlanCard
        plan={{
          description: "Starting execution",
          isStreaming: true,
          sourceCallId: "call-update-plan",
          tasks: [
            {
              id: "task-1",
              status: "in_progress",
              title: "Implement the UI",
            },
          ],
          title: "Execution Plan",
        }}
      />
    );

    expect(
      container.querySelector('[data-slot="floating-plan-card"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="floating-plan-card"]')
    ).toHaveClass("backdrop-blur-sm");
    expect(screen.getByText("Execution Plan")).toBeInTheDocument();
    expect(screen.getByText("Implement the UI")).toBeInTheDocument();
  });
});
