import { useState } from "react";
import { render, screen, within } from "@testing-library/react";
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

  it("switches categories without losing the draft and remembers the last selection per category", async () => {
    const user = userEvent.setup();

    render(
      <PromptComposer
        agents={[
          {
            description: "Review code changes",
            id: "reviewer",
            label: "Code Reviewer",
          },
          {
            description: "Break work into tasks",
            id: "planner",
            label: "Planner",
          },
        ]}
        onSubmit={vi.fn()}
        workflows={[
          {
            description: "Draft a design doc",
            id: "design",
            label: "Write Design",
          },
          {
            description: "Prepare a review package",
            id: "review",
            label: "Request Review",
          },
        ]}
      />
    );

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Keep this text"
    );
    await user.click(screen.getByRole("button", { name: /code reviewer/i }));
    await user.click(await screen.findByRole("menuitem", { name: /planner/i }));

    await user.click(screen.getByRole("button", { name: "Workflows" }));
    expect(screen.getByRole("textbox", { name: /prompt/i })).toHaveValue(
      "Keep this text"
    );
    expect(screen.getByRole("button", { name: /write design/i })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Agents" }));
    expect(screen.getByRole("button", { name: /planner/i })).toBeInTheDocument();
  });

  it("renders both picker groups and keeps disabled items unselectable", async () => {
    const user = userEvent.setup();

    render(
      <PromptComposer
        agents={agents}
        onSubmit={vi.fn()}
        workflows={[
          {
            description: "Draft a design doc",
            id: "design",
            label: "Write Design",
          },
          {
            description: "Unavailable",
            disabled: true,
            id: "locked",
            label: "Release Workflow",
          },
        ]}
      />
    );

    await user.click(screen.getByRole("button", { name: /code reviewer/i }));
    const menu = await screen.findByRole("menu");

    expect(within(menu).getByText("Agents")).toBeInTheDocument();
    expect(within(menu).getByText("Workflows")).toBeInTheDocument();
    expect(
      within(menu).getByRole("menuitem", { name: /write design/i })
    ).toBeInTheDocument();
    expect(
      within(menu)
        .getByText("Release Workflow")
        .closest("[aria-disabled='true']")
    ).toBeInTheDocument();
  });
});
