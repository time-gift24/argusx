import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ConversationView } from "./conversation-view";

type ChatStoreState = {
  messages: Record<string, Array<{ id: string; role: "user" | "assistant" | "system"; content: string; createdAt: number }>>;
  turns: Record<string, Array<{ id: string; createdAt: number }>>;
  scrollToBottomSignal: Record<string, number>;
};

const mocks = vi.hoisted(() => {
  const sessionId = "session-1";
  const scrollToBottom = vi.fn();
  const state: ChatStoreState = {
    messages: { [sessionId]: [] },
    turns: { [sessionId]: [] },
    scrollToBottomSignal: { [sessionId]: 0 },
  };

  return {
    sessionId,
    scrollToBottom,
    state,
  };
});

const SESSION_ID = mocks.sessionId;

vi.mock("use-stick-to-bottom", () => {
  const StickToBottomRoot = ({
    children,
    className,
    ...props
  }: {
    children: ReactNode;
    className?: string;
  }) => (
    <div className={className} {...props}>
      {children}
    </div>
  );
  const StickToBottom = Object.assign(StickToBottomRoot, {
    Content: ({
      children,
      className,
      ...props
    }: {
      children: ReactNode;
      className?: string;
    }) => (
      <div className={className} {...props}>
        {children}
      </div>
    ),
  });

  return {
    StickToBottom,
    useStickToBottomContext: () => ({
      isAtBottom: true,
      scrollToBottom: mocks.scrollToBottom,
    }),
  };
});

vi.mock("@/lib/stores/chat-store", () => ({
  useChatStore: (selector: (state: ChatStoreState) => unknown) =>
    selector(mocks.state),
}));

vi.mock("./agent-turn-card", () => ({
  AgentTurnCard: () => <div data-testid="agent-turn-card" />,
}));

vi.mock("./turn-checkpoint", () => ({
  TurnCheckpoint: () => <div data-testid="turn-checkpoint" />,
}));

describe("ConversationView scroll sync", () => {
  beforeEach(() => {
    mocks.scrollToBottom.mockReset();
    mocks.state.messages = { [SESSION_ID]: [] };
    mocks.state.turns = { [SESSION_ID]: [] };
    mocks.state.scrollToBottomSignal = { [SESSION_ID]: 0 };
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      cb(16.7);
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
  });

  it("forces scroll when scroll signal increments", () => {
    const { rerender } = render(<ConversationView sessionId={SESSION_ID} />);
    expect(mocks.scrollToBottom).toHaveBeenCalledTimes(0);

    mocks.state.scrollToBottomSignal[SESSION_ID] = 1;
    rerender(<ConversationView sessionId={SESSION_ID} />);

    expect(mocks.scrollToBottom).toHaveBeenCalledTimes(2);
  });
});
