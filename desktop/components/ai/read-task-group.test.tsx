import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { ReadTaskGroup } from "@/components/ai/read-task-group";

describe("ReadTaskGroup", () => {
  it("renders a collapsed Summary trigger and reveals compact items on expand", async () => {
    const user = userEvent.setup();

    render(
      <ReadTaskGroup
        items={[
          {
            callId: "call-read-1",
            inputSummary: "src/lib.rs",
            name: "read",
            outputSummary: "pub fn render()",
            status: "success",
          },
          {
            callId: "call-grep-1",
            errorSummary: "pattern not found",
            inputSummary: "src",
            name: "grep",
            status: "failed",
          },
        ]}
      />
    );

    const trigger = screen.getByRole("button", { name: "Summary" });

    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("src/lib.rs")).not.toBeInTheDocument();
    expect(screen.queryByText("pattern not found")).not.toBeInTheDocument();

    await user.click(trigger);

    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("read")).toBeInTheDocument();
    expect(screen.getByText("grep")).toBeInTheDocument();
    expect(screen.getByText("src/lib.rs")).toBeInTheDocument();
    expect(screen.getByText("pub fn render()")).toBeInTheDocument();
    expect(screen.getByText("pattern not found")).toBeInTheDocument();
  });
});
