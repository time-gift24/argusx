"use client";

import type { PromptInputMessage } from "@/components/ai-elements/prompt-input";

import {
  Attachment,
  AttachmentPreview,
  AttachmentRemove,
  Attachments,
} from "@/components/ai-elements/attachments";
import {
  ModelSelector,
  ModelSelectorContent,
  ModelSelectorEmpty,
  ModelSelectorGroup,
  ModelSelectorInput,
  ModelSelectorItem,
  ModelSelectorList,
  ModelSelectorLogo,
  ModelSelectorLogoGroup,
  ModelSelectorName,
  ModelSelectorTrigger,
} from "@/components/ai-elements/model-selector";
import {
  PromptInput,
  PromptInputActionAddAttachments,
  PromptInputActionMenu,
  PromptInputActionMenuContent,
  PromptInputActionMenuTrigger,
  PromptInputBody,
  PromptInputButton,
  PromptInputFooter,
  PromptInputProvider,
  PromptInputSubmit,
  PromptInputTextarea,
  PromptInputTools,
  usePromptInputAttachments,
} from "@/components/ai-elements/prompt-input";
import { startAgentTurn, type ProviderId } from "@/lib/api/chat";
import { useChatStore } from "@/lib/stores/chat-store";
import { useLlmRuntimeConfigStore } from "@/lib/stores/llm-runtime-config-store";
import { CheckIcon, SearchIcon } from "lucide-react";
import type { FocusEvent } from "react";
import { memo, useCallback, useEffect, useMemo, useState } from "react";

import type { FileUIPart } from "@/types";

type ModelOption = {
  id: string;
  name: string;
  provider: ProviderId;
  providerLabel: string;
  providerIcon: string;
};

const PROVIDER_LABEL: Record<ProviderId, string> = {
  bigmodel: "BigModel",
  openai: "OpenAI",
  anthropic: "Anthropic",
};

interface AttachmentItemProps {
  attachment: FileUIPart & { id: string };
  onRemove: (id: string) => void;
}

const AttachmentItem = memo(({ attachment, onRemove }: AttachmentItemProps) => {
  const handleRemove = useCallback(
    () => onRemove(attachment.id),
    [onRemove, attachment.id]
  );
  return (
    <Attachment data={attachment} key={attachment.id} onRemove={handleRemove}>
      <AttachmentPreview />
      <AttachmentRemove />
    </Attachment>
  );
});

AttachmentItem.displayName = "AttachmentItem";

interface ModelItemProps {
  m: ModelOption;
  selectedModel: string;
  onSelect: (id: string) => void;
}

const ModelItem = memo(({ m, selectedModel, onSelect }: ModelItemProps) => {
  const handleSelect = useCallback(() => onSelect(m.id), [onSelect, m.id]);
  return (
    <ModelSelectorItem key={m.id} onSelect={handleSelect} value={m.id}>
      <ModelSelectorLogo provider={m.providerIcon} />
      <ModelSelectorName>{m.name}</ModelSelectorName>
      <ModelSelectorLogoGroup>
        <ModelSelectorLogo provider={m.providerIcon} />
      </ModelSelectorLogoGroup>
      {selectedModel === m.id ? (
        <CheckIcon className="ml-auto size-4" />
      ) : (
        <div className="ml-auto size-4" />
      )}
    </ModelSelectorItem>
  );
});

ModelItem.displayName = "ModelItem";

const PromptInputAttachmentsDisplay = () => {
  const attachments = usePromptInputAttachments();

  const handleRemove = useCallback(
    (id: string) => attachments.remove(id),
    [attachments]
  );

  if (attachments.files.length === 0) {
    return null;
  }

  return (
    <Attachments variant="inline">
      {attachments.files.map((attachment) => (
        <AttachmentItem
          attachment={attachment}
          key={attachment.id}
          onRemove={handleRemove}
        />
      ))}
    </Attachments>
  );
};

export function ChatPromptInput() {
  const {
    currentSessionId,
    sessions,
    addMessage,
    updateSessionStatus,
    ensureAgentTurn,
    requestScrollToBottom,
  } = useChatStore();
  const {
    availableModels,
    selected,
    setSelected,
    error: runtimeConfigError,
  } = useLlmRuntimeConfigStore();
  const [modelSelectorOpen, setModelSelectorOpen] = useState(false);
  const [submitStatus, setSubmitStatus] = useState<
    "submitted" | "streaming" | "ready" | "error"
  >("ready");
  const modelOptions = useMemo<ModelOption[]>(
    () =>
      availableModels.map((item) => ({
        id: `${item.provider}:${item.model}`,
        name: item.model,
        provider: item.provider,
        providerLabel: PROVIDER_LABEL[item.provider],
        providerIcon: item.provider === "bigmodel" ? "zhipuai" : item.provider,
      })),
    [availableModels]
  );
  const hasAvailableModels = modelOptions.length > 0;
  const selectedOption = useMemo(() => {
    if (!selected) {
      return modelOptions[0] ?? null;
    }
    return (
      modelOptions.find(
        (item) =>
          item.provider === selected.provider && item.name === selected.model
      ) ??
      modelOptions[0] ??
      null
    );
  }, [selected, modelOptions]);
  const groupedModelOptions = useMemo(() => {
    const byProvider: Record<string, ModelOption[]> = {};
    for (const option of modelOptions) {
      byProvider[option.providerLabel] ??= [];
      byProvider[option.providerLabel].push(option);
    }
    return Object.entries(byProvider);
  }, [modelOptions]);
  const currentSession = useMemo(
    () => sessions.find((session) => session.id === currentSessionId),
    [sessions, currentSessionId]
  );
  const status: "submitted" | "streaming" | "ready" | "error" =
    submitStatus === "error"
      ? "error"
      : currentSession?.status === "wait-input" ||
          currentSession?.status === "await-input"
        ? "ready"
        : "streaming";
  const [isPromptFocused, setIsPromptFocused] = useState(false);

  useEffect(() => {
    if (selectedOption) {
      setSelected(selectedOption.provider, selectedOption.name);
    }
  }, [selectedOption, setSelected]);

  const handleModelSelect = useCallback((id: string) => {
    const option = modelOptions.find((item) => item.id === id);
    if (option) {
      setSelected(option.provider, option.name);
    }
    setModelSelectorOpen(false);
  }, [modelOptions, setSelected]);

  const handlePromptFocusCapture = useCallback(() => {
    setIsPromptFocused(true);
  }, []);

  const handlePromptBlurCapture = useCallback(
    (event: FocusEvent<HTMLFormElement>) => {
      const nextFocusTarget = event.relatedTarget as Node | null;
      if (!event.currentTarget.contains(nextFocusTarget)) {
        setIsPromptFocused(false);
      }
    },
    []
  );

  useEffect(() => {
    if (
      !isPromptFocused ||
      !currentSessionId ||
      currentSession?.status !== "await-input"
    ) {
      return;
    }
    updateSessionStatus(currentSessionId, "wait-input");
  }, [
    isPromptFocused,
    currentSessionId,
    currentSession?.status,
    updateSessionStatus,
  ]);

  const handleSubmit = useCallback(
    async (message: PromptInputMessage) => {
      const hasText = Boolean(message.text);
      const provider = selectedOption?.provider;
      const selectedModel = selectedOption?.name;

      if (!hasText || !currentSessionId || !provider || !selectedModel) {
        return;
      }

      const messageId = addMessage(currentSessionId, {
        role: "user",
        content: message.text,
      });
      requestScrollToBottom(currentSessionId);

      setSubmitStatus("submitted");
      updateSessionStatus(currentSessionId, "thinking");

      try {
        if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
          await new Promise<void>((resolve) => {
            window.requestAnimationFrame(() => resolve());
          });
        }
        const { turnId } = await startAgentTurn({
          sessionId: currentSessionId,
          input: message.text,
          provider,
          model: selectedModel,
          attachments: message.files,
        });
        ensureAgentTurn(currentSessionId, turnId, messageId);
        setSubmitStatus("streaming");
      } catch (error) {
        console.error("Failed to start agent turn", error);
        updateSessionStatus(currentSessionId, "wait-input");
        setSubmitStatus("error");
      }
    },
    [
      currentSessionId,
      addMessage,
      ensureAgentTurn,
      requestScrollToBottom,
      selectedOption,
      updateSessionStatus,
    ]
  );

  return (
    <PromptInputProvider>
      <PromptInput
        className="w-full"
        globalDrop
        inputGroupClassName="rounded-2xl has-[textarea]:rounded-2xl has-data-[align=block-end]:rounded-2xl has-data-[align=block-start]:rounded-2xl border-white/55 bg-background/80 p-1.5 shadow-[0_1px_0_rgba(255,255,255,0.72)_inset,0_14px_36px_-24px_rgba(15,23,42,0.65),0_1px_3px_rgba(15,23,42,0.2)] backdrop-blur-2xl transition-[background-color,border-color,box-shadow] duration-200 motion-reduce:transition-none has-[[data-slot=input-group-control]:focus-visible]:border-primary/60 has-[[data-slot=input-group-control]:focus-visible]:ring-primary/25 dark:border-white/12 dark:bg-background/55 dark:shadow-[0_16px_36px_-24px_rgba(2,6,23,0.9),0_1px_2px_rgba(2,6,23,0.55)]"
        multiple
        onBlurCapture={handlePromptBlurCapture}
        onFocusCapture={handlePromptFocusCapture}
        onSubmit={handleSubmit}
      >
        <PromptInputAttachmentsDisplay />
        <PromptInputBody>
          <PromptInputTextarea
            className="px-3 py-2.5"
            disabled={!hasAvailableModels}
            placeholder={
              hasAvailableModels
                ? "发送消息..."
                : "配置提供商模型以启用对话"
            }
          />
        </PromptInputBody>
        <PromptInputFooter className="flex-wrap items-center gap-2">
          <PromptInputTools className="flex-wrap gap-1.5">
            <PromptInputActionMenu>
              <PromptInputActionMenuTrigger
                className="transition-colors duration-200 motion-reduce:transition-none"
                disabled={!hasAvailableModels}
              />
              <PromptInputActionMenuContent>
                <PromptInputActionAddAttachments />
              </PromptInputActionMenuContent>
            </PromptInputActionMenu>
            <PromptInputButton
              className="transition-colors duration-200 motion-reduce:transition-none"
              disabled={!hasAvailableModels}
            >
              <SearchIcon size={16} />
              <span>搜索</span>
            </PromptInputButton>
            <ModelSelector
              onOpenChange={setModelSelectorOpen}
              open={modelSelectorOpen}
            >
              <ModelSelectorTrigger asChild>
                <PromptInputButton
                  className={`max-w-full transition-colors duration-200 motion-reduce:transition-none ${
                    hasAvailableModels
                      ? ""
                      : "border-destructive/60 text-destructive hover:bg-destructive/10"
                  }`}
                  disabled={!hasAvailableModels}
                >
                  {selectedOption?.providerIcon && (
                    <ModelSelectorLogo provider={selectedOption.providerIcon} />
                  )}
                  {selectedOption?.name && (
                    <ModelSelectorName className="max-w-28">{selectedOption.name}</ModelSelectorName>
                  )}
                  {!selectedOption && (
                    <ModelSelectorName className="max-w-44 text-destructive">
                      无可用模型
                    </ModelSelectorName>
                  )}
                </PromptInputButton>
              </ModelSelectorTrigger>
              <ModelSelectorContent>
                <ModelSelectorInput placeholder="搜索模型..." />
                <ModelSelectorList>
                  <ModelSelectorEmpty>未找到模型。</ModelSelectorEmpty>
                  {groupedModelOptions.map(([group, options]) => (
                    <ModelSelectorGroup heading={group} key={group}>
                      {options.map((m) => (
                        <ModelItem
                          key={m.id}
                          m={m}
                          onSelect={handleModelSelect}
                          selectedModel={selectedOption?.id ?? ""}
                        />
                      ))}
                    </ModelSelectorGroup>
                  ))}
                </ModelSelectorList>
              </ModelSelectorContent>
            </ModelSelector>
          </PromptInputTools>
          <PromptInputSubmit
            className="transition-colors duration-200 motion-reduce:transition-none"
            disabled={!hasAvailableModels}
            status={status}
          />
          {!hasAvailableModels && (
            <p className="w-full text-xs text-red-500">
              无可用模型。请配置提供商设置。
            </p>
          )}
          {runtimeConfigError && (
            <p className="w-full text-xs text-red-500">{runtimeConfigError}</p>
          )}
        </PromptInputFooter>
      </PromptInput>
    </PromptInputProvider>
  );
}
