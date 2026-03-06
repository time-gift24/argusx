import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { StreamPlayground } from "./stream-playground";

afterEach(() => {
  vi.useRealTimers();
});

describe("StreamPlayground", () => {
  it("starts appending streamed tokens when a run starts", () => {
    vi.useFakeTimers();
    render(<StreamPlayground />);

    fireEvent.click(screen.getByRole("button", { name: "Finish Run" }));
    expect(screen.queryByText(/streamed token arrived/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Start Run" }));

    act(() => {
      vi.advanceTimersByTime(1_600);
    });

    expect(screen.getAllByText(/streamed token arrived/i).length).toBeGreaterThan(
      0
    );
  });
});
