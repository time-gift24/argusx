import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Reasoning } from "@/components/ai/reasoning";

const globalsCssPath = resolve(process.cwd(), "app/globals.css");
const streamdownConfigPath = resolve(
  process.cwd(),
  "components/ai/streamdown-config.ts"
);
const streamdownCodePath = resolve(
  process.cwd(),
  "components/ai/streamdown-code.tsx"
);

describe("Reasoning", () => {
  it("renders markdown through the stream item shell without custom streamdown chrome", () => {
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
      "text-[14px]"
    );
    expect(screen.getByRole("button", { name: /reasoning/i }).className).toContain(
      "leading-5"
    );
    expect(container.querySelector(".ai-streamdown")).not.toBeInTheDocument();
  });

  it("renders fenced code without the custom code shell", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"```ts\nconst value = 1;\n```"}
      </Reasoning>
    );

    expect(container).toHaveTextContent("const value = 1;");
    expect(
      container.querySelector('[data-streamdown="custom-code-panel"]')
    ).not.toBeInTheDocument();
  });

  it("renders math and mermaid source without extra plugins", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {
          [
            "Inline math: $E = mc^2$",
            "",
            "```mermaid",
            "graph TD",
            "  A[Start] --> B[Done]",
            "```",
          ].join("\n")
        }
      </Reasoning>
    );

    expect(container).toHaveTextContent("Inline math: $E = mc^2$");
    expect(container).toHaveTextContent("graph TD");
    expect(container.querySelector(".katex")).not.toBeInTheDocument();
    expect(
      container.querySelector('[data-streamdown="mermaid-block-actions"]')
    ).not.toBeInTheDocument();
  });

  it("keeps stream-item body content on the 14px scale", () => {
    const { container } = render(
      <Reasoning isRunning runKey={1}>
        {"Body copy"}
      </Reasoning>
    );

    expect(
      container.querySelector('[data-slot="stream-item-content"]')
    ).toHaveClass("text-[14px]");
    expect(
      container.querySelector('[data-slot="stream-item-content"]')
    ).toHaveClass("leading-5");
    expect(
      container
        .querySelector('[data-slot="stream-item-trigger"]')
        ?.querySelector("svg")
    ).not.toHaveClass("size-[10px]");
  });

  it("marks the legacy streamdown customization layer as deprecated reference code", () => {
    const globalsCss = readFileSync(globalsCssPath, "utf8");
    const streamdownConfig = readFileSync(streamdownConfigPath, "utf8");
    const streamdownCode = readFileSync(streamdownCodePath, "utf8");

    expect(globalsCss).toMatch(/deprecated/i);
    expect(streamdownConfig).toMatch(/@deprecated/);
    expect(streamdownCode).toMatch(/@deprecated/);
  });
});
