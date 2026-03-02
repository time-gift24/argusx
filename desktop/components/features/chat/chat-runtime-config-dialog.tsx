"use client";

import { useState } from "react";
import type {
  LlmRuntimeConfig,
  ProviderId,
  ProviderRuntimeConfig,
} from "@/lib/api/chat";
import { useLlmRuntimeConfigStore } from "@/lib/stores/llm-runtime-config-store";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PlusIcon, Trash2Icon } from "lucide-react";

interface ChatRuntimeConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const PROVIDERS: Array<{ id: ProviderId; label: string }> = [
  { id: "bigmodel", label: "BigModel" },
  { id: "openai", label: "OpenAI" },
  { id: "anthropic", label: "Anthropic" },
];

const EMPTY_PROVIDER: ProviderRuntimeConfig = {
  apiKey: "",
  baseUrl: "",
  models: [""],
  headers: [],
};

function cloneConfig(config: LlmRuntimeConfig): LlmRuntimeConfig {
  return {
    defaultProvider: config.defaultProvider,
    providers: {
      bigmodel: {
        ...config.providers.bigmodel,
        models: [...config.providers.bigmodel.models],
        headers: [...config.providers.bigmodel.headers],
      },
      openai: {
        ...config.providers.openai,
        models: [...config.providers.openai.models],
        headers: [...config.providers.openai.headers],
      },
      anthropic: {
        ...config.providers.anthropic,
        models: [...config.providers.anthropic.models],
        headers: [...config.providers.anthropic.headers],
      },
    },
  };
}

export function ChatRuntimeConfigDialog({
  open,
  onOpenChange,
}: ChatRuntimeConfigDialogProps) {
  const { config, saveConfig, loading, error } = useLlmRuntimeConfigStore();
  const [draft, setDraft] = useState<LlmRuntimeConfig>(() => cloneConfig(config));
  const [activeTab, setActiveTab] = useState<ProviderId>("bigmodel");
  const [submitError, setSubmitError] = useState<string | null>(null);

  const patchProvider = (
    provider: ProviderId,
    updater: (current: ProviderRuntimeConfig) => ProviderRuntimeConfig
  ) => {
    setDraft((current) => ({
      ...current,
      providers: {
        ...current.providers,
        [provider]: updater(current.providers[provider] ?? { ...EMPTY_PROVIDER }),
      },
    }));
  };

  const handleSave = async () => {
    setSubmitError(null);
    try {
      const normalized = await saveConfig(draft);
      setDraft(cloneConfig(normalized));
      onOpenChange(false);
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : "Failed to save config");
    }
  };

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>LLM Runtime Configuration</DialogTitle>
          <DialogDescription>
            Configure provider API keys, base URLs, models, and custom headers. Changes are runtime-only.
          </DialogDescription>
        </DialogHeader>

        <Tabs
          className="min-h-0"
          onValueChange={(value) => setActiveTab(value as ProviderId)}
          value={activeTab}
        >
          <TabsList>
            {PROVIDERS.map((provider) => (
              <TabsTrigger key={provider.id} value={provider.id}>
                {provider.label}
              </TabsTrigger>
            ))}
          </TabsList>

          {PROVIDERS.map((provider) => {
            const cfg = draft.providers[provider.id];
            return (
              <TabsContent className="space-y-4" key={provider.id} value={provider.id}>
                <div className="grid gap-2">
                  <Label htmlFor={`${provider.id}-api-key`}>API Key</Label>
                  <Input
                    id={`${provider.id}-api-key`}
                    onChange={(event) =>
                      patchProvider(provider.id, (current) => ({
                        ...current,
                        apiKey: event.target.value,
                      }))
                    }
                    placeholder="Required"
                    type="password"
                    value={cfg.apiKey}
                  />
                </div>

                <div className="grid gap-2">
                  <Label htmlFor={`${provider.id}-base-url`}>Base URL</Label>
                  <Input
                    id={`${provider.id}-base-url`}
                    onChange={(event) =>
                      patchProvider(provider.id, (current) => ({
                        ...current,
                        baseUrl: event.target.value,
                      }))
                    }
                    placeholder="Required"
                    value={cfg.baseUrl}
                  />
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>Models</Label>
                    <Button
                      onClick={() =>
                        patchProvider(provider.id, (current) => ({
                          ...current,
                          models: [...current.models, ""],
                        }))
                      }
                      size="sm"
                      type="button"
                      variant="outline"
                    >
                      <PlusIcon className="size-3.5" />
                      Add
                    </Button>
                  </div>
                  <div className="space-y-1.5">
                    {cfg.models.length === 0 ? (
                      <p className="text-muted-foreground text-xs">At least one model is required.</p>
                    ) : null}
                    {(cfg.models.length === 0 ? [""] : cfg.models).map((model, index) => (
                      <div className="flex items-center gap-2" key={`${provider.id}-model-${index}`}>
                        <Input
                          onChange={(event) =>
                            patchProvider(provider.id, (current) => {
                              const next = [...current.models];
                              next[index] = event.target.value;
                              return { ...current, models: next };
                            })
                          }
                          placeholder="e.g. gpt-4o"
                          value={model}
                        />
                        <Button
                          onClick={() =>
                            patchProvider(provider.id, (current) => ({
                              ...current,
                              models: current.models.filter((_, i) => i !== index),
                            }))
                          }
                          size="icon-sm"
                          type="button"
                          variant="ghost"
                        >
                          <Trash2Icon className="size-3.5" />
                        </Button>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>Custom Headers</Label>
                    <Button
                      onClick={() =>
                        patchProvider(provider.id, (current) => ({
                          ...current,
                          headers: [...current.headers, { key: "", value: "" }],
                        }))
                      }
                      size="sm"
                      type="button"
                      variant="outline"
                    >
                      <PlusIcon className="size-3.5" />
                      Add
                    </Button>
                  </div>
                  <div className="space-y-1.5">
                    {cfg.headers.length === 0 ? (
                      <p className="text-muted-foreground text-xs">Optional.</p>
                    ) : null}
                    {cfg.headers.map((header, index) => (
                      <div className="grid grid-cols-[1fr_1fr_auto] items-center gap-2" key={`${provider.id}-header-${index}`}>
                        <Input
                          onChange={(event) =>
                            patchProvider(provider.id, (current) => {
                              const next = [...current.headers];
                              next[index] = { ...next[index], key: event.target.value };
                              return { ...current, headers: next };
                            })
                          }
                          placeholder="Header Key"
                          value={header.key}
                        />
                        <Input
                          onChange={(event) =>
                            patchProvider(provider.id, (current) => {
                              const next = [...current.headers];
                              next[index] = { ...next[index], value: event.target.value };
                              return { ...current, headers: next };
                            })
                          }
                          placeholder="Header Value"
                          value={header.value}
                        />
                        <Button
                          onClick={() =>
                            patchProvider(provider.id, (current) => ({
                              ...current,
                              headers: current.headers.filter((_, i) => i !== index),
                            }))
                          }
                          size="icon-sm"
                          type="button"
                          variant="ghost"
                        >
                          <Trash2Icon className="size-3.5" />
                        </Button>
                      </div>
                    ))}
                  </div>
                </div>
              </TabsContent>
            );
          })}
        </Tabs>

        {(submitError || error) && (
          <p className="text-destructive text-xs">{submitError ?? error}</p>
        )}

        <DialogFooter>
          <Button onClick={handleSave} type="button" disabled={loading}>
            Save Runtime Config
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
