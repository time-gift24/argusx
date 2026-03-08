import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("desktop turn client", () => {
  it("loads the active chat thread from the tauri command layer", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { loadActiveChatThread } = await import("./chat");
    vi.mocked(invoke).mockResolvedValue([
      {
        assistantText: "Existing answer",
        error: null,
        latestPlan: null,
        prompt: "Existing prompt",
        reasoningText: "",
        status: "completed",
        toolCalls: [],
        turnId: "turn-existing",
      },
    ]);

    const out = await loadActiveChatThread();

    expect(out).toEqual([
      {
        assistantText: "Existing answer",
        error: null,
        latestPlan: null,
        prompt: "Existing prompt",
        reasoningText: "",
        status: "completed",
        toolCalls: [],
        turnId: "turn-existing",
      },
    ]);
    expect(invoke).toHaveBeenCalledWith("load_active_chat_thread");
  });

  it("delegates startTurn to the tauri command layer", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { startTurn } = await import("./chat");
    vi.mocked(invoke).mockResolvedValue({ turnId: "turn-1" });

    const out = await startTurn({
      prompt: "hello",
      targetKind: "agent",
      targetId: "sre-agent",
    });

    expect(out.turnId).toBe("turn-1");
    expect(invoke).toHaveBeenCalledWith("start_turn", {
      input: {
        prompt: "hello",
        targetKind: "agent",
        targetId: "sre-agent",
      },
    });
  });

  it("forwards plan-updated events from tauri subscriptions unchanged", async () => {
    const payload = {
      data: {
        sourceCallId: "call-1",
        tasks: [
          { id: "task-1", status: "in_progress", title: "Write failing test" },
        ],
        title: "Execution Plan",
      },
      turnId: "turn-1",
      type: "plan-updated",
    };
    const callback = vi.fn();
    const { listen } = await import("@tauri-apps/api/event");
    const { subscribe } = await import("./chat");

    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      handler({ payload } as never);
      return () => undefined;
    });

    await subscribe(callback);

    expect(callback).toHaveBeenCalledWith(payload);
  });

  it("delegates resolveTurnPermission to the tauri command layer", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { resolveTurnPermission } = await import("./chat");

    await resolveTurnPermission("turn-1", "perm-1", "allow");

    expect(invoke).toHaveBeenCalledWith("resolve_turn_permission", {
      decision: "allow",
      requestId: "perm-1",
      turnId: "turn-1",
    });
  });
});
