"use client";

import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  deleteProviderProfile,
  listProviderProfiles,
  saveProviderProfile,
  setDefaultProviderProfile,
  testProviderProfile,
  type ProviderKind,
  type ProviderProfileSummary,
  type SaveProviderProfileInput,
} from "@/lib/provider-settings";

type FormState = {
  id?: string;
  providerKind: ProviderKind;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  isDefault: boolean;
};

const ZAI_DEFAULTS = {
  baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4/",
  model: "glm-5",
  name: "Z.ai",
} as const;

function emptyForm(providerKind: ProviderKind, isDefault: boolean): FormState {
  const defaults =
    providerKind === "zai"
      ? ZAI_DEFAULTS
      : { baseUrl: "", model: "", name: "" };

  return {
    apiKey: "",
    baseUrl: defaults.baseUrl,
    isDefault,
    model: defaults.model,
    name: defaults.name,
    providerKind,
  };
}

function formFromProfile(profile: ProviderProfileSummary): FormState {
  return {
    apiKey: "",
    baseUrl: profile.baseUrl,
    id: profile.id,
    isDefault: profile.isDefault,
    model: profile.model,
    name: profile.name,
    providerKind: profile.providerKind,
  };
}

export function ProviderSettingsDialog() {
  const [open, setOpen] = useState(false);
  const [profiles, setProfiles] = useState<ProviderProfileSummary[]>([]);
  const [form, setForm] = useState<FormState>(
    emptyForm("openai_compatible", true)
  );
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    void refreshProfiles();
  }, [open]);

  const refreshProfiles = async () => {
    setIsLoading(true);
    setError(null);

    try {
      const nextProfiles = await listProviderProfiles();
      setProfiles(nextProfiles);
      setForm((current) =>
        current.id
          ? current
          : emptyForm(current.providerKind, nextProfiles.length === 0)
      );
    } catch (cause) {
      setError(readableMessage(cause, "无法加载 provider 配置。"));
    } finally {
      setIsLoading(false);
    }
  };

  const handleFieldChange = <Key extends keyof FormState>(
    key: Key,
    value: FormState[Key]
  ) => {
    setForm((current) => ({
      ...current,
      [key]: value,
    }));
  };

  const handleNewOpenAiCompatibleProfile = () => {
    setError(null);
    setStatusMessage(null);
    setForm(emptyForm("openai_compatible", profiles.length === 0));
  };

  const handleZaiProfile = () => {
    setError(null);
    setStatusMessage(null);
    const existingZai = profiles.find((profile) => profile.providerKind === "zai");
    setForm(
      existingZai
        ? formFromProfile(existingZai)
        : emptyForm("zai", profiles.length === 0)
    );
  };

  const handleEditProfile = (profile: ProviderProfileSummary) => {
    setError(null);
    setStatusMessage(null);
    setForm(formFromProfile(profile));
  };

  const handleSave = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    setStatusMessage(null);

    const trimmedApiKey = form.apiKey.trim();
      const payload: SaveProviderProfileInput = {
        baseUrl: form.baseUrl.trim(),
        isDefault: form.isDefault,
        model: form.model.trim(),
        name: form.name.trim(),
        providerKind: form.providerKind,
        ...(form.id ? { id: form.id } : {}),
        ...(trimmedApiKey ? { apiKey: trimmedApiKey } : {}),
      };

    try {
      await saveProviderProfile(payload);
      setStatusMessage("配置已保存。");
      const nextProfiles = await listProviderProfiles();
      setProfiles(nextProfiles);
      setForm(emptyForm(form.providerKind, nextProfiles.length === 0));
    } catch (cause) {
      setError(readableMessage(cause, "保存 provider 配置失败。"));
    } finally {
      setIsSaving(false);
    }
  };

  const handleSetDefault = async (profileId: string) => {
    setError(null);
    setStatusMessage(null);

    try {
      await setDefaultProviderProfile(profileId);
      setStatusMessage("默认 provider 已更新。");
        const nextProfiles = await listProviderProfiles();
        setProfiles(nextProfiles);
        setForm((current) => {
          if (!current.id) {
          return emptyForm(current.providerKind, nextProfiles.length === 0);
          }

        const currentProfile = nextProfiles.find(
          (profile) => profile.id === current.id
        );
        return currentProfile ? formFromProfile(currentProfile) : current;
      });
    } catch (cause) {
      setError(readableMessage(cause, "更新默认 provider 失败。"));
    }
  };

  const handleDelete = async (profileId: string) => {
    setError(null);
    setStatusMessage(null);

    try {
      await deleteProviderProfile(profileId);
      setStatusMessage("配置已删除。");
      const nextProfiles = await listProviderProfiles();
      setProfiles(nextProfiles);
      setForm((current) =>
        current.id === profileId
          ? emptyForm(current.providerKind, nextProfiles.length === 0)
          : current
      );
    } catch (cause) {
      setError(readableMessage(cause, "删除 provider 配置失败。"));
    }
  };

  const handleTestConnection = async () => {
    setError(null);
    setStatusMessage(null);

    if (!form.apiKey.trim()) {
      setError("测试连接需要填写 API Key。");
      return;
    }

    setIsTesting(true);

    try {
      const result = await testProviderProfile({
        apiKey: form.apiKey.trim(),
        baseUrl: form.baseUrl.trim(),
        model: form.model.trim(),
        providerKind: form.providerKind,
      });
      setStatusMessage(result.message);
    } catch (cause) {
      setError(readableMessage(cause, "测试连接失败。"));
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={setOpen}
    >
      <DialogTrigger asChild>
        <Button
          size="sm"
          type="button"
          variant="outline"
        >
          Provider 配置
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>Provider 配置</DialogTitle>
          <DialogDescription>
            管理 OpenAI-compatible profiles，并选择一个全局默认项给聊天使用。
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
          <section className="flex min-h-0 flex-col gap-3 rounded-xl border border-border/70 bg-muted/20 p-3">
            <div className="flex items-center justify-between gap-2">
              <div>
                <p className="text-sm font-medium">已保存配置</p>
                <p className="text-xs text-muted-foreground">
                  支持单个 Z.ai 配置和多个 OpenAI-compatible profiles
                </p>
              </div>
              <div className="flex flex-wrap justify-end gap-2">
                <Button
                  onClick={handleZaiProfile}
                  size="sm"
                  type="button"
                  variant="secondary"
                >
                  配置 Z.ai
                </Button>
                <Button
                  onClick={handleNewOpenAiCompatibleProfile}
                  size="sm"
                  type="button"
                  variant="secondary"
                >
                  新增 OpenAI-compatible
                </Button>
              </div>
            </div>

            {isLoading ? (
              <p className="text-sm text-muted-foreground">正在加载配置...</p>
            ) : profiles.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                还没有保存任何 provider profile。
              </p>
            ) : (
              <div className="flex flex-col gap-2">
                {profiles.map((profile) => (
                  <div
                    className="rounded-lg border border-border/70 bg-background p-3"
                    key={profile.id}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <p className="truncate text-sm font-medium">
                            {profile.name}
                          </p>
                          <span className="rounded-full border border-border/70 px-2 py-0.5 text-[10px] text-muted-foreground">
                            {providerKindLabel(profile.providerKind)}
                          </span>
                          {profile.isDefault ? (
                            <span className="rounded-full bg-foreground px-2 py-0.5 text-[10px] font-medium text-background">
                              默认
                            </span>
                          ) : null}
                        </div>
                        <p className="truncate text-xs text-muted-foreground">
                          {profile.model}
                        </p>
                        <p className="truncate text-xs text-muted-foreground">
                          {profile.baseUrl}
                        </p>
                      </div>
                      <div className="flex shrink-0 flex-wrap justify-end gap-2">
                        <Button
                          aria-label={`编辑 ${profile.name}`}
                          onClick={() => handleEditProfile(profile)}
                          size="sm"
                          type="button"
                          variant="outline"
                        >
                          编辑
                        </Button>
                        {!profile.isDefault ? (
                          <Button
                            onClick={() => void handleSetDefault(profile.id)}
                            size="sm"
                            type="button"
                            variant="secondary"
                          >
                            设为默认
                          </Button>
                        ) : null}
                        <Button
                          disabled={profile.isDefault}
                          onClick={() => void handleDelete(profile.id)}
                          size="sm"
                          type="button"
                          variant="ghost"
                        >
                          删除
                        </Button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>

          <section className="rounded-xl border border-border/70 bg-background p-3">
            <form
              className="flex flex-col gap-3"
              onSubmit={(event) => void handleSave(event)}
            >
              <div className="flex items-center justify-between gap-2">
                <div>
                  <p className="text-sm font-medium">
                    {form.id ? "编辑配置" : "新建配置"}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {form.isDefault ? "保存后会作为默认 provider 使用" : "保存为备用 provider profile"}
                  </p>
                </div>
                <span className="rounded-full border border-border/70 px-2 py-0.5 text-[10px] text-muted-foreground">
                  {providerKindLabel(form.providerKind)}
                </span>
              </div>

              <div className="grid gap-3">
                <div className="grid gap-1.5">
                  <Label htmlFor="provider-profile-kind">Provider 类型</Label>
                  <Input
                    aria-label="Provider 类型"
                    id="provider-profile-kind"
                    readOnly
                    value={providerKindLabel(form.providerKind)}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="provider-profile-name">名称</Label>
                  <Input
                    id="provider-profile-name"
                    onChange={(event) => handleFieldChange("name", event.target.value)}
                    value={form.name}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="provider-profile-base-url">Base URL</Label>
                  <Input
                    id="provider-profile-base-url"
                    onChange={(event) =>
                      handleFieldChange("baseUrl", event.target.value)
                    }
                    value={form.baseUrl}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="provider-profile-model">Model</Label>
                  <Input
                    id="provider-profile-model"
                    onChange={(event) => handleFieldChange("model", event.target.value)}
                    value={form.model}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="provider-profile-api-key">API Key</Label>
                  <Input
                    id="provider-profile-api-key"
                    onChange={(event) => handleFieldChange("apiKey", event.target.value)}
                    placeholder={form.id ? "留空表示保持当前密钥不变" : undefined}
                    type="password"
                    value={form.apiKey}
                  />
                </div>
              </div>

              {error ? (
                <p
                  className="text-sm text-destructive"
                  role="alert"
                >
                  {error}
                </p>
              ) : null}
              {statusMessage ? (
                <p className="text-sm text-muted-foreground">{statusMessage}</p>
              ) : null}

              <div className="flex flex-wrap gap-2">
                <Button
                  disabled={isSaving}
                  type="submit"
                >
                  {isSaving ? "保存中..." : "保存配置"}
                </Button>
                <Button
                  disabled={isTesting}
                  onClick={() => void handleTestConnection()}
                  type="button"
                  variant="secondary"
                >
                  {isTesting ? "测试中..." : "测试连接"}
                </Button>
              </div>
            </form>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function readableMessage(cause: unknown, fallback: string): string {
  if (cause instanceof Error && cause.message) {
    return cause.message;
  }

  if (typeof cause === "string" && cause.trim()) {
    return cause;
  }

  return fallback;
}

function providerKindLabel(providerKind: ProviderKind): string {
  return providerKind === "zai" ? "Z.ai" : "OpenAI-compatible";
}
