import userEvent from "@testing-library/user-event";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

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
      autoCloseDelayMs={0}
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
});
