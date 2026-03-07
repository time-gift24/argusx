import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("provider settings client", () => {
  it("delegates profile save to the tauri command layer", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { saveProviderProfile } = await import("./provider-settings");
    vi.mocked(invoke).mockResolvedValue({
      baseUrl: "https://openrouter.ai/api/v1/",
      id: "profile-1",
      isDefault: true,
      model: "openai/gpt-4.1-mini",
      name: "OpenRouter",
      providerKind: "openai_compatible",
    });

    const out = await saveProviderProfile({
      apiKey: "sk-openrouter",
      baseUrl: "https://openrouter.ai/api/v1/",
      isDefault: true,
      model: "openai/gpt-4.1-mini",
      name: "OpenRouter",
    });

    expect(out.id).toBe("profile-1");
    expect(invoke).toHaveBeenCalledWith("save_provider_profile", {
      input: {
        apiKey: "sk-openrouter",
        baseUrl: "https://openrouter.ai/api/v1/",
        isDefault: true,
        model: "openai/gpt-4.1-mini",
        name: "OpenRouter",
      },
    });
  });
});
