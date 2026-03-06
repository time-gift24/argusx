import userEvent from "@testing-library/user-event";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
} from "@/components/ai/stream-item";

function Harness({
  isRunning,
  runKey,
}: {
  isRunning: boolean;
  runKey: number;
}) {
  return (
    <StreamItem
      autoCloseDelayMs={10}
      defaultOpen={false}
      defaultOpenWhenRunning
      isRunning={isRunning}
      runKey={runKey}
    >
      <StreamItemTrigger label="Reasoning" status="Thinking" />
      <StreamItemContent>stream body</StreamItemContent>
    </StreamItem>
  );
}

afterEach(() => {
  vi.useRealTimers();
});

describe("StreamItem", () => {
  it("opens automatically when a run starts", () => {
    const { rerender } = render(<Harness isRunning={false} runKey={1} />);

    expect(screen.queryByText("stream body")).not.toBeInTheDocument();

    rerender(<Harness isRunning runKey={1} />);

    expect(screen.getByText("stream body")).toBeInTheDocument();
  });

  it("does not auto-reopen after a manual collapse in the same run", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness isRunning runKey={1} />);

    await user.click(screen.getByRole("button", { name: /reasoning/i }));
    expect(screen.queryByText("stream body")).not.toBeInTheDocument();

    rerender(<Harness isRunning runKey={1} />);

    expect(screen.queryByText("stream body")).not.toBeInTheDocument();
  });

  it("resets manual-collapse memory when the run key changes", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness isRunning runKey={1} />);

    await user.click(screen.getByRole("button", { name: /reasoning/i }));
    expect(screen.queryByText("stream body")).not.toBeInTheDocument();

    rerender(<Harness isRunning runKey={2} />);

    expect(screen.getByText("stream body")).toBeInTheDocument();
  });

  it("renders a dedicated shimmer overlay while the item is running", () => {
    const { container, rerender } = render(<Harness isRunning runKey={1} />);

    expect(
      container.querySelector('[data-slot="stream-item-shimmer"]')
    ).toBeInTheDocument();

    rerender(<Harness isRunning={false} runKey={1} />);

    expect(
      container.querySelector('[data-slot="stream-item-shimmer"]')
    ).not.toBeInTheDocument();
  });

  it("does not auto-close after the user reopens the item during the same run", async () => {
    vi.useFakeTimers();
    const { rerender } = render(<Harness isRunning runKey={1} />);
    const trigger = screen.getByRole("button", { name: /reasoning/i });

    fireEvent.click(trigger);
    fireEvent.click(trigger);

    rerender(<Harness isRunning={false} runKey={1} />);

    act(() => {
      vi.advanceTimersByTime(10);
    });

    expect(screen.getByText("stream body")).toBeInTheDocument();
  });

  it("keeps runtime-open provenance across run key changes", () => {
    vi.useFakeTimers();
    const { rerender } = render(<Harness isRunning runKey={1} />);

    rerender(<Harness isRunning runKey={2} />);
    rerender(<Harness isRunning={false} runKey={2} />);

    act(() => {
      vi.advanceTimersByTime(10);
    });

    expect(screen.queryByText("stream body")).not.toBeInTheDocument();
  });
});
