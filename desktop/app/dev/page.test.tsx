import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import DevPage from "./page";

describe("DevPage", () => {
  it("renders the dev directory and defaults to the prompt composer showcase", () => {
    render(<DevPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Dev" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Prompt Composer" })
    ).toHaveAttribute("data-state", "active");
    expect(
      screen.getByRole("heading", { level: 2, name: "Prompt Composer" })
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open showcase/i })).toHaveAttribute(
      "href",
      "/chat"
    );
  });

  it("switches the right panel when a different showcase is selected", async () => {
    const user = userEvent.setup();

    render(<DevPage />);

    await user.click(
      screen.getByRole("button", { name: "Streamdown Playground" })
    );

    expect(
      screen.getByRole("heading", { level: 2, name: "Streamdown Playground" })
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open showcase/i })).toHaveAttribute(
      "href",
      "/dev/streamdown"
    );
    expect(screen.getByText("Markdown")).toBeInTheDocument();
    expect(screen.getByText("Mermaid")).toBeInTheDocument();
  });
});
