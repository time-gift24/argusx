import { fireEvent, render, screen } from "@testing-library/react";
import { act } from "react";
import { describe, expect, it, vi } from "vitest";
import { WrenchIcon } from "lucide-react";

import { RuntimeProcessSection } from "./runtime-process-section";

describe("RuntimeProcessSection", () => {
  it("renders left-aligned header with shimmer timer and controlled collapse", () => {
    vi.useFakeTimers();
    const onOpenChange = vi.fn();

    const { rerender } = render(
      <RuntimeProcessSection
        icon={WrenchIcon}
        isStreaming
        label="Running tools..."
        onOpenChange={onOpenChange}
        open={false}
      >
        <div>tools content</div>
      </RuntimeProcessSection>
    );

    expect(screen.getByText("Running tools...")).toBeInTheDocument();
    expect(screen.getByText("0s")).toBeInTheDocument();
    expect(screen.queryByText("tools content")).not.toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(screen.getByText("2s")).toBeInTheDocument();

    fireEvent.click(
      screen
        .getByText("Running tools...")
        .closest('[data-slot="collapsible-trigger"]')!
    );
    expect(onOpenChange).toHaveBeenCalledWith(true);

    rerender(
      <RuntimeProcessSection
        icon={WrenchIcon}
        isStreaming={false}
        label="Tools"
        onOpenChange={onOpenChange}
        open={false}
      >
        <div>tools content</div>
      </RuntimeProcessSection>
    );

    expect(screen.getByText("Tools")).toBeInTheDocument();
    expect(screen.getByText("2s")).toBeInTheDocument();
  });
});
