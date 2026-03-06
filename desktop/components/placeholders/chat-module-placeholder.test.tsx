import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChatModulePlaceholder } from "@/components/placeholders/chat-module-placeholder";

describe("ChatModulePlaceholder", () => {
  it("renders redesign placeholder copy for the page surface", () => {
    render(<ChatModulePlaceholder variant="page" />);

    expect(screen.getByText("对话模块已移除")).toBeInTheDocument();
    expect(screen.getByText("等待新的桌面工作台设计")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("renders compact placeholder copy for the sidebar surface", () => {
    render(<ChatModulePlaceholder variant="sidebar" />);

    expect(screen.getByText("右侧面板占位")).toBeInTheDocument();
    expect(screen.getByText("此区域保留给后续重设计。")).toBeInTheDocument();
  });
});
