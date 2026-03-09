import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HomePage from "./page";

const startTurn = vi.fn();
const cancelTurn = vi.fn();
const listAgentProfiles = vi.fn();
const loadActiveChatThread = vi.fn();
const resolveTurnPermission = vi.fn();
const subscribe = vi.fn();

vi.mock("@/lib/chat", () => ({
  useTurn: () => ({
    cancelTurn,
    listAgentProfiles,
    loadActiveChatThread,
    resolveTurnPermission,
    startTurn,
    subscribe,
  }),
}));

describe("HomePage", () => {
  beforeEach(() => {
    startTurn.mockReset();
    cancelTurn.mockReset();
    listAgentProfiles.mockReset();
    loadActiveChatThread.mockReset();
    resolveTurnPermission.mockReset();
    subscribe.mockReset();
    listAgentProfiles.mockResolvedValue([
      {
        description: "Break ambiguous work into concrete steps",
        id: "builtin-main",
        label: "Planner",
      },
    ]);
    loadActiveChatThread.mockResolvedValue([]);
    resolveTurnPermission.mockResolvedValue(undefined);
    subscribe.mockResolvedValue(() => {});
  });

  it("renders the chat workspace directly at the root route", async () => {
    render(<HomePage />);

    expect(await screen.findByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.queryByText("对话模块已移除")).not.toBeInTheDocument();
  });

  it("does not render the retired placeholder copy", async () => {
    render(<HomePage />);

    await screen.findByRole("button", { name: "Agents" });
    expect(screen.queryByText("等待新的桌面工作台设计")).not.toBeInTheDocument();
    expect(screen.queryByText("这里不会再展示消息流")).not.toBeInTheDocument();
  });
});
