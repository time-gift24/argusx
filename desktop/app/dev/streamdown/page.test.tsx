import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/components/ai/streamdown-plugins", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("@/components/ai/streamdown-plugins")>();

  return {
    ...actual,
    sharedMermaidPlugin: {
      ...actual.sharedMermaidPlugin,
      getMermaid: () => ({
        initialize: () => undefined,
        render: vi.fn().mockResolvedValue({
          svg: '<svg aria-label="Mermaid chart"><text>Rendered Mermaid</text></svg>',
        }),
      }),
    },
    sharedStreamdownPlugins: {
      ...actual.sharedStreamdownPlugins,
      mermaid: {
        ...actual.sharedMermaidPlugin,
        getMermaid: () => ({
          initialize: () => undefined,
          render: vi.fn().mockResolvedValue({
            svg: '<svg aria-label="Mermaid chart"><text>Rendered Mermaid</text></svg>',
          }),
        }),
      },
    },
  };
});

import StreamdownPage from "./page";

afterEach(() => {
  vi.clearAllMocks();
});

describe("StreamdownPage", () => {
  it("renders the streamdown playground samples", async () => {
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
    expect(document.querySelector(".katex")).toBeInTheDocument();
    expect(container.querySelector(".ai-streamdown")).toBeInTheDocument();
    await waitFor(() => {
      expect(
        document.querySelector('[data-streamdown="mermaid-block"]')
      ).toBeInTheDocument();
    });
    expect(
      document.querySelector('[data-streamdown="mermaid-block-actions"]')
    ).toBeInTheDocument();
    expect(
      document.querySelector('[data-streamdown="custom-code-panel"]')
    ).toBeInTheDocument();
    expect(
      document
        .querySelector('[data-streamdown="custom-code-panel"]')
        ?.closest('[data-slot="stream-item"]')
    ).toBeInTheDocument();
    expect(
      document.querySelector(
        '[data-streamdown="code-language-icon"][data-language="javascript"]'
      )
    ).toBeInTheDocument();
    expect(
      document.querySelector(
        '[data-streamdown="custom-code-panel"] [data-streamdown="code-block-header"]'
      )
    ).not.toBeInTheDocument();
    expect(
      document.querySelector(
        '[data-streamdown="custom-code-panel"] [data-streamdown="code-block-body"] [data-streamdown="code-block-actions"]'
      )
    ).toBeInTheDocument();
    expect(screen.getAllByText("Ready").length).toBeGreaterThan(0);
    expect(
      document.querySelector('[data-slot="runtime-mermaid-surface"]')
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Start" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Next Demo" })
    ).toBeInTheDocument();
  });
});
