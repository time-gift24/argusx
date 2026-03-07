import { render, screen } from "@testing-library/react";
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
});
