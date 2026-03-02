import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatPage } from "./chat-page";

type ChatStoreState = {
  sessions: Array<{ id: string }>;
  currentSessionId: string | null;
  createSession: () => void;
};

type RuntimeConfigStoreState = {
  availableModels: Array<{ provider: string; model: string }>;
  loading: boolean;
  bootstrap: () => Promise<void>;
};

const mocks = vi.hoisted(() => {
  const listenAgentStreamMock = vi.fn();
  const bootstrapMock = vi.fn();
  const createSessionMock = vi.fn();
  const applyAgentStreamEnvelopeMock = vi.fn();
  const chatStoreState: ChatStoreState = {
    sessions: [{ id: "session-1" }],
    currentSessionId: "session-1",
    createSession: createSessionMock,
  };
  const useChatStoreMock = vi.fn(() => chatStoreState);
  (
    useChatStoreMock as unknown as {
      getState: () => { applyAgentStreamEnvelope: () => void };
    }
  ).getState = () => ({
    applyAgentStreamEnvelope: applyAgentStreamEnvelopeMock,
  });
  const runtimeConfigStoreState: RuntimeConfigStoreState = {
    availableModels: [],
    loading: false,
    bootstrap: bootstrapMock,
  };
  return {
    applyAgentStreamEnvelopeMock,
    bootstrapMock,
    chatStoreState,
    createSessionMock,
    listenAgentStreamMock,
    runtimeConfigStoreState,
    useChatStoreMock,
  };
});

vi.mock("@/lib/api/chat", () => ({
  listenAgentStream: mocks.listenAgentStreamMock,
}));

vi.mock("@/lib/stores/chat-store", () => ({
  useChatStore: mocks.useChatStoreMock,
}));

vi.mock("@/lib/stores/llm-runtime-config-store", () => ({
  useLlmRuntimeConfigStore: (selector: (state: RuntimeConfigStoreState) => unknown) =>
    selector(mocks.runtimeConfigStoreState),
}));

vi.mock("./conversation-view", () => ({
  ConversationView: ({ sessionId }: { sessionId: string }) => (
    <div data-testid="conversation-view">{sessionId}</div>
  ),
}));

vi.mock("./chat-session-bar", () => ({
  ChatSessionBar: () => <div data-testid="chat-session-bar" />,
}));

vi.mock("./chat-runtime-config-dialog", () => ({
  ChatRuntimeConfigDialog: ({ open }: { open: boolean }) => (
    <div data-open={open ? "true" : "false"} data-testid="runtime-config-dialog" />
  ),
}));

describe("ChatPage", () => {
  beforeEach(() => {
    mocks.runtimeConfigStoreState.availableModels = [];
    mocks.runtimeConfigStoreState.loading = false;
    mocks.runtimeConfigStoreState.bootstrap = mocks.bootstrapMock;
    mocks.listenAgentStreamMock.mockResolvedValue(() => undefined);
    mocks.bootstrapMock.mockReset();
    mocks.bootstrapMock.mockResolvedValue(undefined);
    mocks.createSessionMock.mockReset();
    mocks.applyAgentStreamEnvelopeMock.mockReset();
    mocks.chatStoreState.sessions = [{ id: "session-1" }];
    mocks.chatStoreState.currentSessionId = "session-1";
  });

  it("shows orange ping hint when no models are available and loading finished", () => {
    mocks.runtimeConfigStoreState.availableModels = [];
    mocks.runtimeConfigStoreState.loading = false;

    const { container } = render(<ChatPage />);

    expect(container.querySelector(".animate-ping")).toBeInTheDocument();

    const button = screen.getByRole("button", { name: "Open runtime config" });
    expect(button.className).toContain("border-orange-500/40");
  });

  it("hides ping hint when models are available", () => {
    mocks.runtimeConfigStoreState.availableModels = [{ provider: "openai", model: "gpt-4o" }];
    mocks.runtimeConfigStoreState.loading = false;

    const { container } = render(<ChatPage />);

    expect(container.querySelector(".animate-ping")).not.toBeInTheDocument();

    const button = screen.getByRole("button", { name: "Open runtime config" });
    expect(button.className).toContain("border-primary/40");
  });

  it("hides ping hint while config is loading", () => {
    mocks.runtimeConfigStoreState.availableModels = [];
    mocks.runtimeConfigStoreState.loading = true;

    const { container } = render(<ChatPage />);

    expect(container.querySelector(".animate-ping")).not.toBeInTheDocument();
  });

  it("opens runtime config dialog when button is clicked", () => {
    const { rerender } = render(<ChatPage />);

    const button = screen.getByRole("button", { name: "Open runtime config" });
    fireEvent.click(button);

    rerender(<ChatPage />);

    expect(screen.getByTestId("runtime-config-dialog")).toHaveAttribute(
      "data-open",
      "true"
    );
  });
});
