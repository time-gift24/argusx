import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getChatMessages,
  getChatTurnSummaries,
  updateChatSession,
} from "@/lib/api/chat";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: () => true,
}));

describe("chat api invoke payload", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("uses camelCase sessionId for getChatMessages", async () => {
    invokeMock.mockResolvedValueOnce([]);

    await getChatMessages("s1", {
      range: "all",
      cursor: 123,
      limit: 456,
    });

    expect(invokeMock).toHaveBeenCalledWith("get_chat_messages", {
      sessionId: "s1",
      range: "all",
      cursor: 123,
      limit: 456,
    });

    const [, payload] = invokeMock.mock.calls[0] as [string, Record<string, unknown>];
    expect(payload).not.toHaveProperty("session_id");
  });

  it("uses camelCase sessionId for getChatTurnSummaries", async () => {
    invokeMock.mockResolvedValueOnce([]);

    await getChatTurnSummaries("s1");

    expect(invokeMock).toHaveBeenCalledWith("get_chat_turn_summaries", {
      sessionId: "s1",
    });

    const [, payload] = invokeMock.mock.calls[0] as [string, Record<string, unknown>];
    expect(payload).not.toHaveProperty("session_id");
  });

  it("sends update payload with session id and title", async () => {
    invokeMock.mockResolvedValueOnce({});

    await updateChatSession("s1", { title: "Renamed" });

    expect(invokeMock).toHaveBeenCalledWith("update_chat_session", {
      payload: {
        id: "s1",
        title: "Renamed",
      },
    });
  });
});
