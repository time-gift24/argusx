import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Reasoning } from "@/components/ai/reasoning";

describe("Reasoning", () => {
  it("renders streamed content through the shared runtime shell", () => {
    render(
      <Reasoning isRunning runKey={1}>
        {"First line\n\n- item"}
      </Reasoning>
    );

    expect(
      screen.getByRole("button", { name: /reasoning/i })
    ).toBeInTheDocument();
    expect(screen.getByText("First line")).toBeInTheDocument();
  });
});
