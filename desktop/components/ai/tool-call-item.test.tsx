import userEvent from "@testing-library/user-event";
import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ToolCallItem } from "@/components/ai/tool-call-item";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("ToolCallItem", () => {
  it("renders fallback summaries for running and completed states", () => {
    const { rerender } = render(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning
        name="shell"
        runKey={1}
      />
    );

    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("cwd: /workspace")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /shell/i }).className).toContain(
      "text-[10px]"
    );
    expect(screen.getByRole("button", { name: /shell/i }).className).toContain(
      "leading-[12px]"
    );
    expect(screen.getByText("Running").className).toContain("text-[10px]");
    expect(screen.getByText("Running").className).toContain("leading-[12px]");

    rerender(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning={false}
        name="shell"
        outputSummary="exit 0"
        runKey={1}
      />
    );

    expect(screen.getByText("Completed")).toBeInTheDocument();
    expect(screen.getByText("exit 0")).toBeInTheDocument();
  });

  it("renders structured code sections with copy and download actions", async () => {
    const user = userEvent.setup();
    const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
    const createObjectURL = vi
      .spyOn(URL, "createObjectURL")
      .mockReturnValue("blob:runtime");
    const revokeObjectURL = vi
      .spyOn(URL, "revokeObjectURL")
      .mockImplementation(() => undefined);
    const anchorClicks: HTMLAnchorElement[] = [];
    const originalCreateElement = document.createElement.bind(document);

    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: {
        writeText: clipboardWriteText,
      },
    });

    vi.spyOn(document, "createElement").mockImplementation((tagName) => {
      const element = originalCreateElement(tagName);

      if (tagName === "a") {
        vi.spyOn(element as HTMLAnchorElement, "click").mockImplementation(
          () => undefined
        );
        anchorClicks.push(element as HTMLAnchorElement);
      }

      return element;
    });

    render(
      <ToolCallItem
        {...({
          name: "shell",
          runKey: 1,
          sections: [
            {
              code: "pwd && ls",
              id: "input",
              label: "Input",
              language: "bash",
            },
          ],
        } as const)}
      />
    );

    await user.click(screen.getByRole("button", { name: /shell/i }));

    const codeRegion = document.querySelector(
      '[data-slot="runtime-code-surface"]'
    ) as HTMLElement | null;
    const actions = document.querySelector(
      '[data-slot="runtime-code-actions"]'
    ) as HTMLElement | null;
    expect(codeRegion).not.toBeNull();
    expect(actions).not.toBeNull();
    expect(codeRegion).toHaveTextContent("pwd && ls");
    expect(codeRegion?.className).toContain("bg-background");
    expect(codeRegion?.className).not.toContain("bg-primary-foreground");
    expect(codeRegion?.querySelector("pre")?.getAttribute("style")).toContain(
      "background-color: transparent;"
    );
    expect(codeRegion?.querySelector("pre")?.getAttribute("style")).not.toContain(
      "opacity:"
    );
    expect(actions?.className).toContain("top-1.5");
    expect(actions?.className).toContain("right-2.5");
    expect(actions?.firstElementChild?.className).toContain("bg-transparent");
    expect(actions?.firstElementChild?.className).toContain("border-0");
    expect(actions?.firstElementChild?.className).toContain("shadow-none");

    const scoped = within(codeRegion!);
    const copyButton = scoped.getByRole("button", { name: "Copy code" });
    const downloadButton = scoped.getByRole("button", { name: "Download code" });

    expect(copyButton.className).toContain("bg-transparent");
    expect(copyButton.className).toContain("border-0");
    expect(copyButton.className).toContain("size-3");
    expect(copyButton.className).toContain("hover:text-primary");
    expect(downloadButton.className).toContain("bg-transparent");
    expect(downloadButton.className).toContain("border-0");
    expect(downloadButton.className).toContain("size-3");
    expect(downloadButton.className).toContain("hover:text-primary");
    expect(copyButton.querySelector("svg")).toHaveAttribute("width", "12");
    expect(downloadButton.querySelector("svg")).toHaveAttribute("width", "12");

    await user.click(copyButton);
    expect(clipboardWriteText).toHaveBeenCalledWith("pwd && ls");

    await user.click(downloadButton);
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(anchorClicks.at(-1)?.download).toBe("input.sh");
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:runtime");
  });
});
