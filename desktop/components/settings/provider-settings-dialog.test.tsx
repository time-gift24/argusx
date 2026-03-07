import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProviderSettingsDialog } from "@/components/settings/provider-settings-dialog";

const providerSettingsApi = vi.hoisted(() => ({
  deleteProviderProfile: vi.fn(),
  listProviderProfiles: vi.fn(),
  saveProviderProfile: vi.fn(),
  setDefaultProviderProfile: vi.fn(),
  testProviderProfile: vi.fn(),
}));

vi.mock("@/lib/provider-settings", () => ({
  deleteProviderProfile: providerSettingsApi.deleteProviderProfile,
  listProviderProfiles: providerSettingsApi.listProviderProfiles,
  saveProviderProfile: providerSettingsApi.saveProviderProfile,
  setDefaultProviderProfile: providerSettingsApi.setDefaultProviderProfile,
  testProviderProfile: providerSettingsApi.testProviderProfile,
}));

describe("ProviderSettingsDialog", () => {
  beforeEach(() => {
    providerSettingsApi.deleteProviderProfile.mockReset();
    providerSettingsApi.listProviderProfiles.mockReset();
    providerSettingsApi.saveProviderProfile.mockReset();
    providerSettingsApi.setDefaultProviderProfile.mockReset();
    providerSettingsApi.testProviderProfile.mockReset();
  });

  it("opens from the header trigger and loads saved profiles", async () => {
    const user = userEvent.setup();
    providerSettingsApi.listProviderProfiles.mockResolvedValue([
      {
        baseUrl: "https://openrouter.ai/api/v1/",
        id: "profile-1",
        isDefault: true,
        model: "openai/gpt-4.1-mini",
        name: "OpenRouter",
        providerKind: "openai_compatible",
      },
    ]);

    render(<ProviderSettingsDialog />);
    await user.click(screen.getByRole("button", { name: "Provider 配置" }));

    expect(await screen.findByRole("dialog", { name: "Provider 配置" })).toBeInTheDocument();
    await waitFor(() => {
      expect(providerSettingsApi.listProviderProfiles).toHaveBeenCalledTimes(1);
    });
    expect(screen.getByText("OpenRouter")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "配置 Z.ai" })).toBeInTheDocument();
  });

  it("creates a new default profile from the dialog form", async () => {
    const user = userEvent.setup();
    providerSettingsApi.listProviderProfiles.mockResolvedValue([]);
    providerSettingsApi.saveProviderProfile.mockResolvedValue({
      baseUrl: "https://openrouter.ai/api/v1/",
      id: "profile-1",
      isDefault: true,
      model: "openai/gpt-4.1-mini",
      name: "OpenRouter",
      providerKind: "openai_compatible",
    });

    render(<ProviderSettingsDialog />);
    await user.click(screen.getByRole("button", { name: "Provider 配置" }));

    await user.type(screen.getByLabelText("名称"), "OpenRouter");
    await user.type(
      screen.getByLabelText("Base URL"),
      "https://openrouter.ai/api/v1/"
    );
    await user.type(screen.getByLabelText("Model"), "openai/gpt-4.1-mini");
    await user.type(screen.getByLabelText("API Key"), "sk-openrouter");
    await user.click(screen.getByRole("button", { name: "保存配置" }));

    expect(providerSettingsApi.saveProviderProfile).toHaveBeenCalledWith({
      apiKey: "sk-openrouter",
      baseUrl: "https://openrouter.ai/api/v1/",
      isDefault: true,
      model: "openai/gpt-4.1-mini",
      name: "OpenRouter",
      providerKind: "openai_compatible",
    });
  });

  it("updates metadata without clearing the stored api key", async () => {
    const user = userEvent.setup();
    providerSettingsApi.listProviderProfiles.mockResolvedValue([
      {
        baseUrl: "https://openrouter.ai/api/v1/",
        id: "profile-1",
        isDefault: true,
        model: "openai/gpt-4.1-mini",
        name: "OpenRouter",
        providerKind: "openai_compatible",
      },
    ]);
    providerSettingsApi.saveProviderProfile.mockResolvedValue({
      baseUrl: "https://openrouter.ai/api/v1/",
      id: "profile-1",
      isDefault: true,
      model: "openai/gpt-4.1",
      name: "OpenRouter Stable",
      providerKind: "openai_compatible",
    });

    render(<ProviderSettingsDialog />);
    await user.click(screen.getByRole("button", { name: "Provider 配置" }));
    await user.click(screen.getByRole("button", { name: "编辑 OpenRouter" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "OpenRouter Stable");
    await user.clear(screen.getByLabelText("Model"));
    await user.type(screen.getByLabelText("Model"), "openai/gpt-4.1");
    await user.click(screen.getByRole("button", { name: "保存配置" }));

    expect(providerSettingsApi.saveProviderProfile).toHaveBeenCalledWith({
      baseUrl: "https://openrouter.ai/api/v1/",
      id: "profile-1",
      isDefault: true,
      model: "openai/gpt-4.1",
      name: "OpenRouter Stable",
      providerKind: "openai_compatible",
    });
  });

  it("prefills the form for a Z.ai profile", async () => {
    const user = userEvent.setup();
    providerSettingsApi.listProviderProfiles.mockResolvedValue([]);

    render(<ProviderSettingsDialog />);
    await user.click(screen.getByRole("button", { name: "Provider 配置" }));
    await user.click(screen.getByRole("button", { name: "配置 Z.ai" }));

    expect(screen.getByLabelText("Provider 类型")).toHaveValue("Z.ai");
    expect(screen.getByLabelText("名称")).toHaveValue("Z.ai");
    expect(screen.getByLabelText("Base URL")).toHaveValue(
      "https://open.bigmodel.cn/api/coding/paas/v4/"
    );
    expect(screen.getByLabelText("Model")).toHaveValue("glm-5");
  });
});
