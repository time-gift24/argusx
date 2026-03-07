import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PlanQueue } from "@/components/ai/plan-queue";

describe("PlanQueue", () => {
  it("renders the latest plan snapshot with completed, in-progress, and pending tasks", () => {
    const { container } = render(
      <PlanQueue
        plan={{
          title: "Execution Plan",
          description: "Starting execution",
          isStreaming: true,
          sourceCallId: "call-1",
          tasks: [
            {
              id: "task-1",
              status: "completed",
              title: "Write failing test",
            },
            {
              id: "task-2",
              status: "in_progress",
              title: "Implement minimal fix",
            },
            {
              id: "task-3",
              status: "pending",
              title: "Run the tests",
            },
          ],
        }}
      />
    );

    expect(screen.getByText("Execution Plan")).toBeInTheDocument();
    expect(screen.getByText("Starting execution")).toBeInTheDocument();
    expect(screen.getByText("Write failing test")).toBeInTheDocument();
    expect(screen.getByText("Implement minimal fix")).toBeInTheDocument();
    expect(screen.getByText("Run the tests")).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="plan-queue"][data-streaming="true"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="plan-queue-item"][data-status="completed"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="plan-queue-item"][data-status="in_progress"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="plan-queue-item"][data-status="pending"]')
    ).toBeInTheDocument();
  });
});
