import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import ChatPage from "./page";

describe("ChatPage", () => {
  it("renders the prompt composer instead of the retired placeholder", () => {
    render(<ChatPage />);

    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Workflows" })).toBeInTheDocument();
    expect(screen.queryByText("对话模块已移除")).not.toBeInTheDocument();
  });
});
