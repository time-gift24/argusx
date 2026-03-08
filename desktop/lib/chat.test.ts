import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("desktop turn client", () => {
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
});
