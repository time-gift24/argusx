import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import StreamdownPage from "./page";

describe("StreamdownPage", () => {
  it("renders the playground samples with default streamdown output", () => {
    const { container } = render(<StreamdownPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Streamdown Playground" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Code Blocks" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Math Equations" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Mermaid Diagram" })
    ).toBeInTheDocument();
    expect(screen.getByText("Capture screenshots")).toBeInTheDocument();
    expect(screen.getByText("Add lint job")).toBeInTheDocument();
    expect(container).toHaveTextContent("Inline math: $E = mc^2$");
    expect(container).toHaveTextContent("graph TD");
    expect(container.querySelector(".katex")).not.toBeInTheDocument();
    expect(container.querySelector(".ai-streamdown")).not.toBeInTheDocument();
    expect(
      document.querySelector('[data-streamdown="custom-code-panel"]')
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Start" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Next Demo" })
    ).toBeInTheDocument();
  });
});
