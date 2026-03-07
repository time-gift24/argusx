import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HomePage from "./page";

const startTurn = vi.fn();
const cancelTurn = vi.fn();
const subscribe = vi.fn();

vi.mock("@/lib/chat", () => ({
  useTurn: () => ({
    cancelTurn,
    startTurn,
    subscribe,
  }),
}));

describe("HomePage", () => {
  beforeEach(() => {
    startTurn.mockReset();
    cancelTurn.mockReset();
    subscribe.mockReset();
    subscribe.mockResolvedValue(() => {});
  });

  it("renders the chat workspace directly at the root route", () => {
    render(<HomePage />);

    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(screen.queryByText("对话模块已移除")).not.toBeInTheDocument();
  });

  it("does not render the retired placeholder copy", () => {
    render(<HomePage />);

    expect(screen.queryByText("等待新的桌面工作台设计")).not.toBeInTheDocument();
    expect(screen.queryByText("这里不会再展示消息流")).not.toBeInTheDocument();
  });
});
