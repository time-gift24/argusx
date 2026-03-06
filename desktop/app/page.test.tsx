import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import DashboardPage from "./page";

describe("DashboardPage", () => {
  it("does not advertise live chat or LLM capability", () => {
    render(<DashboardPage />);

    expect(screen.queryByText(/LLM对话能力/)).not.toBeInTheDocument();
    expect(screen.queryByText(/开始新对话/)).not.toBeInTheDocument();
    expect(screen.queryByText(/配置您的模型/)).not.toBeInTheDocument();
    expect(screen.queryByText(/AI Agent交互体验|AI Agent 交互体验/)).not.toBeInTheDocument();
  });

  it("links the SOP module entry to /sop/annotation", () => {
    const { container } = render(<DashboardPage />);

    expect(container.querySelector('a[href="/sop/annotation"]')).toBeTruthy();
    expect(container.querySelector('a[href="/annotation"]')).toBeFalsy();
  });
});
