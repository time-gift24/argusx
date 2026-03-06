import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ToolCallItem } from "@/components/ai/tool-call-item";

describe("ToolCallItem", () => {
  it("renders fallback summaries for running and completed states", () => {
    const { rerender } = render(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning
        name="shell"
        runKey={1}
      />
    );

    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("cwd: /workspace")).toBeInTheDocument();

    rerender(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning={false}
        name="shell"
        outputSummary="exit 0"
        runKey={1}
      />
    );

    expect(screen.getByText("Completed")).toBeInTheDocument();
    expect(screen.getByText("exit 0")).toBeInTheDocument();
  });
});
