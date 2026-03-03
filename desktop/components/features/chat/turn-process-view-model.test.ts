import { describe, expect, it } from "vitest";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import { buildTurnProcessVM } from "./turn-process-view-model";

const createTurn = (overrides?: Partial<AgentTurnVM>): AgentTurnVM => ({
  id: "turn-1",
  sessionId: "session-1",
  createdAt: 1,
  updatedAt: 1,
  status: "streaming",
  assistantText: "",
  reasoning: {
    isStreaming: true,
    isExpanded: false,
    preview: "Streaming reasoning...",
    text: "Thinking",
    charCount: 8,
    truncated: false,
    updatedAt: 1,
    status: "streaming",
  },
  tools: [],
  queue: { items: [] },
  terminal: {
    stdout: "",
    stderr: "",
    output: "",
    isStreaming: false,
    updatedAt: 1,
  },
  lastSeq: 1,
  ...overrides,
});

describe("buildTurnProcessVM", () => {
  it("sets reasoning/tools/terminal defaultOpen to false and includes header labels", () => {
    const vm = buildTurnProcessVM(
      createTurn({
        queue: {
          items: [
            {
              callId: "call-1",
              status: "running",
              toolName: "web.search",
              updatedAt: 2,
            },
          ],
        },
        tools: [
          {
            callId: "call-1",
            toolName: "web.search",
            state: "input-streaming",
            updatedAt: 2,
          },
        ],
        terminal: {
          stdout: "line-1",
          stderr: "",
          output: "line-1",
          isStreaming: true,
          updatedAt: 2,
        },
      })
    );

    const reasoning = vm.sections.find((section) => section.key === "reasoning");
    const tools = vm.sections.find((section) => section.key === "tools");
    const terminal = vm.sections.find((section) => section.key === "terminal");

    expect(reasoning?.defaultOpen).toBe(false);
    expect(reasoning?.headerLabel).toBe("Thinking...");

    expect(tools?.defaultOpen).toBe(false);
    expect(tools?.headerLabel).toBe("Running tools...");
    expect(tools?.headerDetail).toBe("web.search");

    expect(terminal?.defaultOpen).toBe(false);
    expect(terminal?.headerLabel).toBe("Running terminal...");
  });

  it("builds tools header detail with +N and compact items", () => {
    const vm = buildTurnProcessVM(
      createTurn({
        reasoning: {
          isStreaming: false,
          isExpanded: false,
          preview: "",
          text: "",
          charCount: 0,
          truncated: false,
          updatedAt: 1,
          status: "completed",
        },
        status: "done",
        queue: {
          items: [
            {
              callId: "call-1",
              status: "completed",
              toolName: "web.search",
              updatedAt: 3,
            },
            {
              callId: "call-2",
              status: "completed",
              toolName: "functions.exec_command",
              updatedAt: 2,
            },
          ],
        },
        tools: [
          {
            callId: "call-1",
            toolName: "web.search",
            state: "output-available",
            updatedAt: 3,
          },
          {
            callId: "call-2",
            toolName: "functions.exec_command",
            state: "output-available",
            updatedAt: 2,
          },
        ],
      })
    );

    const tools = vm.sections.find((section) => section.key === "tools");

    expect(tools?.headerDetail).toBe("web.search +1");
    expect(tools?.compactItems).toEqual([
      { id: "call-1", label: "web.search", status: "Completed" },
      {
        id: "call-2",
        label: "functions.exec_command",
        status: "Completed",
      },
    ]);
  });
});
