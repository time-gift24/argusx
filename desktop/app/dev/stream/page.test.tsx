import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import StreamPage from "./page";

describe("StreamPage", () => {
  it("renders the stream playground samples", () => {
    render(<StreamPage />);

    expect(screen.getByRole("button", { name: "Start Run" })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "Reasoning" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "Running Tool" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "Completed Tool" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "Manual Collapse" })
    ).toBeInTheDocument();
  });
});
