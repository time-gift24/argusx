import { readFileSync } from "node:fs";
import { resolve } from "node:path";

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
      "text-[12px]"
    );
    expect(screen.getByRole("button", { name: /reasoning/i }).className).toContain(
      "leading-[14px]"
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
        container.querySelector('[data-streamdown="code-block"]')
      ).toBeInTheDocument();
    });

    expect(
      container.querySelector('[data-streamdown="code-block-actions"]')
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
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="inline-code"\] \{[\s\S]*font-size: 10px;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code \{[\s\S]*font-size: 10px;/s);
  });

  it("removes streamdown block borders and left-aligns code and mermaid content", () => {
    const globalsCss = readFileSync(globalsCssPath, "utf8");

    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block"\],\s*\.ai-streamdown \[data-streamdown="mermaid-block"\] \{[\s\S]*border: 0;[\s\S]*padding: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-header"\] span \{\s*margin-left: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] \{[\s\S]*padding: 0\.375rem 0 0\.375rem 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre \{\s*margin: 0;\s*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code \{\s*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*font-size: 10px !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*line-height: 12px !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*width: 1\.25rem !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*margin-right: 0\.5rem !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*text-align: left !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="code-block-body"\] pre > code > span::before,\s*\.ai-streamdown \[data-streamdown="code-block-body"\] code > span::before \{[\s\S]*padding: 0 !important;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid-block"\] > div:first-child > span \{\s*margin-left: 0;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid-block"\] > div:last-child \{[\s\S]*border: 0;[\s\S]*background: transparent;/s);
    expect(globalsCss).toMatch(/\.ai-streamdown \[data-streamdown="mermaid"\] \[role="img"\] \{\s*justify-content: flex-start;/s);
  });
});
