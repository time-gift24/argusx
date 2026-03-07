import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { PromptComposer } from "@/components/ai/prompt-composer";

const agents = [
  {
    description: "Review code changes",
    id: "reviewer",
    label: "Code Reviewer",
  },
];

const workflows = [
  {
    description: "Draft a design doc",
    id: "design",
    label: "Write Design",
  },
];

describe("PromptComposer", () => {
  it("renders the docked shell with category controls and a disabled submit button", () => {
    render(
      <PromptComposer
        agents={agents}
        onSubmit={vi.fn()}
        workflows={workflows}
      />
    );

    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Workflows" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /send/i })).toBeDisabled();
    expect(screen.getByText("Code Reviewer")).toBeInTheDocument();
  });

  it("submits on Enter and preserves newline behavior with Shift+Enter", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(
      <PromptComposer
        agents={agents}
        onSubmit={onSubmit}
        workflows={workflows}
      />
    );

    const textarea = screen.getByRole("textbox", { name: /prompt/i });

    await user.type(textarea, "Review this change");
    await user.keyboard("{Enter}");

    expect(onSubmit).toHaveBeenCalledWith({
      category: "agent",
      draft: "Review this change",
      selectionId: "reviewer",
    });

    await user.clear(textarea);
    await user.type(textarea, "Line one");
    await user.keyboard("{Shift>}{Enter}{/Shift}");

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(textarea).toHaveValue("Line one\n");
  });

  it("supports a controlled draft value", () => {
    function Harness() {
      const [value, setValue] = useState("Existing prompt");

      return (
        <PromptComposer
          agents={agents}
          onSubmit={vi.fn()}
          onValueChange={setValue}
          value={value}
          workflows={workflows}
        />
      );
    }

    render(<Harness />);

    expect(screen.getByRole("textbox", { name: /prompt/i })).toHaveValue(
      "Existing prompt"
    );
  });
});
