import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import userEvent from "@testing-library/user-event";
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

import { Reasoning } from "@/components/ai/reasoning";

const globalsCssPath = resolve(process.cwd(), "app/globals.css");

afterEach(() => {
  vi.clearAllMocks();
});

describe("Reasoning", () => {
  it("renders streamed content through the shared runtime shell", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"First line\n\n- item"}
      </Reasoning>
    );

    expect(
      screen.getByRole("button", { name: /reasoning/i })
    ).toBeInTheDocument();
    expect(screen.getByText("First line")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reasoning/i }).className).toContain(
      "text-[10px]"
    );
    expect(screen.getByRole("button", { name: /reasoning/i }).className).toContain(
      "leading-[12px]"
    );
    expect(container.querySelector(".ai-streamdown")).toBeInTheDocument();
  });

  it("renders fenced code blocks through official streamdown code surfaces", async () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"```ts\nconst value = 1;\n```"}
      </Reasoning>
    );

    await waitFor(() => {
      expect(
        container.querySelector('[data-streamdown="custom-code-panel"]')
      ).toBeInTheDocument();
    });

    const viewport = container.querySelector(
      '[data-slot="stream-item-viewport"]'
    );
    const languageLabel = screen.getByText("TypeScript");

    expect(languageLabel).toBeInTheDocument();
    expect(languageLabel.className).not.toContain("uppercase");
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(viewport).toHaveAttribute("data-state", "closed");
    expect(
      container.querySelector('[data-streamdown="code-language-icon"][data-language="ts"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector(
        '[data-streamdown="custom-code-panel"] [data-streamdown="code-block-header"]'
      )
    ).not.toBeInTheDocument();
    expect(
      container.querySelector('[data-streamdown="code-block"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-streamdown="code-block-actions"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector(
        '[data-streamdown="custom-code-panel"] [data-streamdown="code-block-body"] [data-streamdown="code-block-actions"]'
      )
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-streamdown="code-block-copy-button"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-streamdown="code-block-download-button"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="runtime-code-surface"]')
    ).not.toBeInTheDocument();
  });

  it("keeps incomplete code fences in a running stream-item shell", async () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"```ts\nconst value = 1;"}
      </Reasoning>
    );

    await waitFor(() => {
      expect(
        container.querySelector('[data-streamdown="custom-code-panel"]')
      ).toBeInTheDocument();
    });

    expect(screen.getByText("TypeScript")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="stream-item-shimmer"]')
    ).toBeInTheDocument();
  });

  it("shows a centered expand button only when a code block can expand", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <Reasoning isRunning={false} open runKey={1}>
        {
          "```ts\nconst first = 1;\nconst second = 2;\nconst third = 3;\nconst fourth = 4;\nconst fifth = 5;\nconst sixth = 6;\nconst seventh = 7;\n```"
        }
      </Reasoning>
    );

    await waitFor(() => {
      expect(
        container.querySelector('[data-streamdown="custom-code-panel"]')
      ).toBeInTheDocument();
    });

    const viewport = container.querySelector(
      '[data-slot="stream-item-viewport"]'
    );
    const expandButton = screen.getByRole("button", { name: /expand code/i });

    expect(viewport).toHaveAttribute("data-state", "closed");
    expect(expandButton).toBeInTheDocument();

    await user.click(expandButton);

    expect(viewport).toHaveAttribute("data-state", "open");
    expect(
      screen.queryByRole("button", { name: /expand code/i })
    ).not.toBeInTheDocument();
  });

  it("keeps unsupported fenced languages on the official code block path", async () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"```customlang\nraw output\n```"}
      </Reasoning>
    );

    await waitFor(() => {
      expect(
        container.querySelector('[data-streamdown="code-block"]')
      ).toBeInTheDocument();
    });

    expect(container).toHaveTextContent("raw output");
    expect(
      container.querySelector('[data-slot="runtime-code-surface"]')
    ).not.toBeInTheDocument();
  });

  it("renders inline math through katex", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"Inline math: $E = mc^2$"}
      </Reasoning>
    );

    expect(container.querySelector(".katex")).toBeInTheDocument();
  });

  it("renders mermaid fences through official streamdown mermaid surfaces", async () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"```mermaid\ngraph TD\n  A[Start] --> B[Done]\n```"}
      </Reasoning>
    );

    await waitFor(() => {
      expect(
        container.querySelector('[data-streamdown="mermaid-block"]')
      ).toBeInTheDocument();
    });

    expect(
      container.querySelector('[data-streamdown="mermaid-block-actions"]')
    ).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="runtime-mermaid-surface"]')
    ).not.toBeInTheDocument();
  });

  it("applies one shared streamdown root class for global styling", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"- item\n- next"}
      </Reasoning>
    );

    expect(container.querySelector(".ai-streamdown")).toBeInTheDocument();
  });

  it("tunes global streamdown spacing and code scale for compact content", () => {
    const globalsCss = readFileSync(globalsCssPath, "utf8");

    expect(globalsCss).toMatch(/\.ai-streamdown > :not\(\[hidden\]\) ~ :not\(\[hidden\]\) \{\s*margin-block-start: 6px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown p \{\s*margin-block: 6px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="unordered-list"\],\s*\.ai-streamdown \[data-streamdown="ordered-list"\] \{\s*margin-block: 6px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="list-item"\] \{\s*padding-block: 2px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="inline-code"\] \{[\s\S]*font-size: 12px;[\s\S]*line-height: 14px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code \{[\s\S]*font-size: 12px;[\s\S]*line-height: 14px;/s);
  });

  it("removes streamdown block borders and left-aligns code and mermaid content", () => {
    const globalsCss = readFileSync(globalsCssPath, "utf8");

    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block"\],\s*\.ai-streamdown \[data-streamdown="mermaid-block"\] \{[\s\S]*border: 0;[\s\S]*padding: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-header"\] span \{\s*margin-left: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] \{[\s\S]*padding: 0\.375rem 0 0\.375rem 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre \{\s*margin: 0;\s*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code \{\s*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*font-size: 12px !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*line-height: 14px !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*width: 1\.25rem !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*margin-right: 0\.5rem !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*text-align: left !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid-block"\] > div:first-child > span \{\s*margin-left: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid-block"\] > div:last-child \{[\s\S]*border: 0;[\s\S]*background: transparent;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid"\] \[role="img"\] \{\s*justify-content: flex-start;/s);
  });

  it("styles the custom code shell with muted backgrounds and viewport limits", () => {
    const globalsCss = readFileSync(globalsCssPath, "utf8");

    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \{[\s\S]*background: color-mix\(in srgb, var\(--muted\) 72%, var\(--background\)\) !important;/s);
    expect(globalsCss).toMatch(/\.dark \.ai-streamdown \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \{[\s\S]*background: color-mix\(in srgb, var\(--muted\) 58%, var\(--background\)\) !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \{[\s\S]*padding: 0\.375rem 2rem 0\.375rem 0\.375rem !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-slot="stream-item-viewport"\]\[data-state="closed"\] \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \{[\s\S]*max-height: calc\(\(14px \* 6\) \+ 0\.75rem\);/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-slot="stream-item-viewport"\]\[data-state="open"\] \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \{[\s\S]*max-height: calc\(\(14px \* 30\) \+ 0\.75rem\);/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="custom-code-panel"\] \[data-streamdown="code-block-body"\] \[data-streamdown="code-block-actions"\] \{[\s\S]*position: absolute;[\s\S]*top: 0\.375rem;[\s\S]*right: 0\.5rem;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-actions"\],\s*\.ai-streamdown \[data-streamdown="mermaid-block-actions"\] \{[\s\S]*gap: 0\.375rem;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-expand-hint"\] \{[\s\S]*position: absolute;[\s\S]*left: 50%;[\s\S]*transform: translateX\(-50%\);/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-expand-button"\] \{[\s\S]*font-size: 12px;[\s\S]*line-height: 14px;/s);
  });
});
