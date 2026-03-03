import { describe, expect, it } from "vitest";
import type { ChatMessage } from "@/lib/stores/chat-store";
import {
  EMPTY_CHAT_MESSAGES,
  selectSessionMessages,
} from "@/lib/stores/chat-selectors";

function createMessage(id: string): ChatMessage {
  return {
    id,
    sessionId: "s1",
    role: "assistant",
    content: "hello",
    createdAt: 1,
  };
}

describe("selectSessionMessages", () => {
  it("returns a stable empty reference for null session id", () => {
    const selector = selectSessionMessages(null);
    const state = { messages: {} };

    const first = selector(state);
    const second = selector(state);

    expect(first).toBe(EMPTY_CHAT_MESSAGES);
    expect(second).toBe(first);
  });

  it("returns a stable empty reference for missing session", () => {
    const selector = selectSessionMessages("missing");
    const state = { messages: {} };

    const first = selector(state);
    const second = selector(state);

    expect(first).toBe(EMPTY_CHAT_MESSAGES);
    expect(second).toBe(first);
  });

  it("returns the original store array when session exists", () => {
    const selector = selectSessionMessages("s1");
    const sessionMessages = [createMessage("m1"), createMessage("m2")];
    const state = { messages: { s1: sessionMessages } };

    const selected = selector(state);

    expect(selected).toBe(sessionMessages);
  });
});
