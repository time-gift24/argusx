import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import HomePage from "./page";

describe("HomePage", () => {
  it("renders the chat workspace directly at the root route", () => {
    render(<HomePage />);

    expect(screen.getByText("对话模块已移除")).toBeInTheDocument();
    expect(
      screen.getByText("等待新的桌面工作台设计")
    ).toBeInTheDocument();
    expect(
      screen.queryByText("欢迎使用 ArgusX")
    ).not.toBeInTheDocument();
  });

  it("does not render the removed dashboard entry content", () => {
    render(<HomePage />);

    expect(screen.queryByText("快速操作")).not.toBeInTheDocument();
    expect(screen.queryByText("入门指南")).not.toBeInTheDocument();
  });
});
