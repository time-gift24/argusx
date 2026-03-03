import { describe, expect, it } from "vitest";

import { trimChatCacheToBudget } from "./chat-cache-budget";

interface TestMessage {
  id: string;
  createdAt: number;
  payload: string;
}

interface TestTurn {
  id: string;
  createdAt: number;
  updatedAt: number;
  payload: string;
}

const LARGE_PAYLOAD = "x".repeat(2048);

const createMessage = (id: string, createdAt: number): TestMessage => ({
  id,
  createdAt,
  payload: LARGE_PAYLOAD,
});

const createTurn = (id: string, createdAt: number): TestTurn => ({
  id,
  createdAt,
  updatedAt: createdAt,
  payload: LARGE_PAYLOAD,
});

describe("trimChatCacheToBudget", () => {
  it("evicts globally oldest records once budget is exceeded", () => {
    const sessions = [
      { id: "s1", updatedAt: 10 },
      { id: "s2", updatedAt: 20 },
    ];
    const messages = {
      s1: [createMessage("m-oldest", 1), createMessage("m-newer", 5)],
      s2: [createMessage("m-s2", 3)],
    };
    const turns = {
      s1: [createTurn("t-s1", 2)],
      s2: [createTurn("t-s2", 4)],
    };

    const baseline = trimChatCacheToBudget(
      sessions,
      messages,
      turns,
      "s2",
      Number.MAX_SAFE_INTEGER
    );
    const trimmed = trimChatCacheToBudget(
      sessions,
      messages,
      turns,
      "s2",
      baseline.estimatedBytes - 1
    );

    expect(trimmed.messages.s1.some((item) => item.id === "m-oldest")).toBe(false);
    expect(trimmed.messages.s1.some((item) => item.id === "m-newer")).toBe(true);
    expect(trimmed.turns.s1.some((item) => item.id === "t-s1")).toBe(true);
  });

  it("does not protect active session when its record is globally oldest", () => {
    const sessions = [
      { id: "active", updatedAt: 10 },
      { id: "other", updatedAt: 20 },
    ];
    const messages = {
      active: [createMessage("m-active-oldest", 1)],
      other: [createMessage("m-other", 2)],
    };
    const turns = {
      active: [] as TestTurn[],
      other: [] as TestTurn[],
    };

    const baseline = trimChatCacheToBudget(
      sessions,
      messages,
      turns,
      "active",
      Number.MAX_SAFE_INTEGER
    );
    const trimmed = trimChatCacheToBudget(
      sessions,
      messages,
      turns,
      "active",
      baseline.estimatedBytes - 1
    );

    expect(trimmed.messages.active).toEqual([]);
    expect(trimmed.messages.other).toHaveLength(1);
  });
});
