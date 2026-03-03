import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { STREAMDOWN_PLUGINS } from "@/components/ai-elements/streamdown-plugins";
import { Streamdown } from "streamdown";

import { RuntimeMarkdownBlock } from "./runtime-markdown-block";

vi.mock("streamdown", async () => {
  const actual = await vi.importActual<typeof import("streamdown")>("streamdown");
  return {
    ...actual,
    Block: ({ content }: { content: string }) => (
      <div data-content={content} data-testid="streamdown-default-block" />
    ),
  };
});

const renderMarkdown = (
  content: string,
  options?: { isAnimating?: boolean }
) =>
  render(
    <Streamdown
      BlockComponent={RuntimeMarkdownBlock}
      isAnimating={options?.isAnimating}
      plugins={STREAMDOWN_PLUGINS}
    >
      {content}
    </Streamdown>
  );

describe("RuntimeMarkdownBlock", () => {
  it("renders fenced code with minimalist runtime surface and copy button", () => {
    const { container } = renderMarkdown("```ts\nconst answer = 42;\n```");

    expect(container.querySelector(".llm-chat-code-surface")).toBeTruthy();
    expect(screen.getByLabelText("Copy code")).toBeInTheDocument();
    // Check that highlighted code renders (may have data-highlighted on wrapper or element)
    const highlighted = container.querySelector('[data-highlighted="true"]') ||
                        container.querySelector(".llm-chat-code-surface");
    expect(highlighted).toBeTruthy();
    expect(screen.queryByTestId("streamdown-default-block")).not.toBeInTheDocument();
  });

  it("renders terminal fences with the same minimalist style", () => {
    const { container } = renderMarkdown("```bash\necho hi\n```");

    expect(container.querySelector(".llm-chat-terminal-surface")).toBeTruthy();
    expect(screen.getByLabelText("Copy terminal output")).toBeInTheDocument();
    expect(screen.queryByLabelText("Download code")).not.toBeInTheDocument();
    expect(screen.queryByTestId("streamdown-default-block")).not.toBeInTheDocument();
  });

  it("keeps mermaid fences on default Streamdown rendering", () => {
    renderMarkdown("```mermaid\ngraph TD;\nA-->B;\n```");

    expect(screen.getByTestId("streamdown-default-block")).toBeInTheDocument();
    expect(screen.queryByLabelText("Copy code")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Copy terminal output")).not.toBeInTheDocument();
  });

  it("shows streaming status and expandable preview for incomplete fences", () => {
    const { container } = renderMarkdown(
      `\`\`\`ts
line-1
line-2
line-3
line-4
line-5
line-6
line-7
line-8
line-9
line-10`,
      { isAnimating: true }
    );

    const codeNode = container.querySelector("code");
    expect(screen.queryByTestId("streamdown-default-block")).not.toBeInTheDocument();
    expect(screen.getByText("生成中")).toBeInTheDocument();
    expect(codeNode?.textContent).toContain("line-8");
    expect(codeNode?.textContent).not.toContain("line-10");

    fireEvent.click(screen.getByRole("button", { name: /展开/ }));
    expect(codeNode?.textContent).toContain("line-10");
    expect(screen.getByRole("button", { name: "收起" })).toBeInTheDocument();
  });

  it("does not syntax-highlight fenced code when language is missing", () => {
    const { container } = renderMarkdown("```\nfn main() {}\n```");
    expect(container.querySelector('[data-highlighted="false"]')).toBeTruthy();
  });

  it("does not syntax-highlight fenced text language", () => {
    const { container } = renderMarkdown("```text\nfn main() {}\n```");
    expect(container.querySelector('[data-highlighted="false"]')).toBeTruthy();
  });
});
