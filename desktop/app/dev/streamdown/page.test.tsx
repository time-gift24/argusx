import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import StreamdownPage from "./page";

describe("StreamdownPage", () => {
  it("renders the streamdown playground samples", () => {
    render(<StreamdownPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Streamdown Playground" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Code Blocks" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Math Equations" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 3, name: "Mermaid Diagram" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Start" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Next Demo" })
    ).toBeInTheDocument();
  });
});
