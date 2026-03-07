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
});
