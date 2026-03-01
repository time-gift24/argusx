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
import { startAgentTurn } from "@/lib/api/chat";
import { useChatStore } from "@/lib/stores/chat-store";
import { CheckIcon, SearchIcon } from "lucide-react";
import type { FocusEvent } from "react";
import { memo, useCallback, useEffect, useMemo, useState } from "react";

const models = [
  {
    chef: "OpenAI",
    chefSlug: "openai",
    id: "gpt-4o",
    name: "GPT-4o",
    providers: ["openai", "azure"],
  },
  {
    chef: "OpenAI",
    chefSlug: "openai",
    id: "gpt-4o-mini",
    name: "GPT-4o Mini",
    providers: ["openai", "azure"],
  },
  {
    chef: "Anthropic",
    chefSlug: "anthropic",
    id: "claude-opus-4-20250514",
    name: "Claude 4 Opus",
    providers: ["anthropic", "azure", "google", "amazon-bedrock"],
  },
  {
    chef: "Anthropic",
    chefSlug: "anthropic",
    id: "claude-sonnet-4-20250514",
    name: "Claude 4 Sonnet",
    providers: ["anthropic", "azure", "google", "amazon-bedrock"],
  },
  {
    chef: "Google",
    chefSlug: "google",
    id: "gemini-2.0-flash-exp",
    name: "Gemini 2.0 Flash",
    providers: ["google"],
  },
];

import type { FileUIPart } from "@/types";

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
  m: (typeof models)[0];
  selectedModel: string;
  onSelect: (id: string) => void;
}

const ModelItem = memo(({ m, selectedModel, onSelect }: ModelItemProps) => {
  const handleSelect = useCallback(() => onSelect(m.id), [onSelect, m.id]);
  return (
    <ModelSelectorItem key={m.id} onSelect={handleSelect} value={m.id}>
      <ModelSelectorLogo provider={m.chefSlug} />
      <ModelSelectorName>{m.name}</ModelSelectorName>
      <ModelSelectorLogoGroup>
        {m.providers.map((provider) => (
          <ModelSelectorLogo key={provider} provider={provider} />
        ))}
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
  } = useChatStore();
  const [model, setModel] = useState<string>(models[0].id);
  const [modelSelectorOpen, setModelSelectorOpen] = useState(false);
  const [submitStatus, setSubmitStatus] = useState<
    "submitted" | "streaming" | "ready" | "error"
  >("ready");

  const selectedModelData = models.find((m) => m.id === model);
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

  const handleModelSelect = useCallback((id: string) => {
    setModel(id);
    setModelSelectorOpen(false);
  }, []);

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

      if (!hasText || !currentSessionId) {
        return;
      }

      const messageId = addMessage(currentSessionId, {
        role: "user",
        content: message.text,
      });

      setSubmitStatus("submitted");
      updateSessionStatus(currentSessionId, "thinking");

      try {
        const { turnId } = await startAgentTurn({
          sessionId: currentSessionId,
          input: message.text,
          model,
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
      model,
      updateSessionStatus,
    ]
  );

  return (
    <PromptInputProvider>
      <PromptInput
        className="w-full"
        globalDrop
        inputGroupClassName="rounded-2xl has-[textarea]:rounded-2xl has-data-[align=block-end]:rounded-2xl has-data-[align=block-start]:rounded-2xl border-white/55 bg-background/80 shadow-[0_1px_0_rgba(255,255,255,0.72)_inset,0_14px_36px_-24px_rgba(15,23,42,0.65),0_1px_3px_rgba(15,23,42,0.2)] backdrop-blur-2xl transition-[background-color,border-color,box-shadow] duration-200 motion-reduce:transition-none has-[[data-slot=input-group-control]:focus-visible]:border-primary/60 has-[[data-slot=input-group-control]:focus-visible]:ring-primary/25 dark:border-white/12 dark:bg-background/55 dark:shadow-[0_16px_36px_-24px_rgba(2,6,23,0.9),0_1px_2px_rgba(2,6,23,0.55)]"
        multiple
        onBlurCapture={handlePromptBlurCapture}
        onFocusCapture={handlePromptFocusCapture}
        onSubmit={handleSubmit}
      >
        <PromptInputAttachmentsDisplay />
        <PromptInputBody>
          <PromptInputTextarea placeholder="Send a message..." />
        </PromptInputBody>
        <PromptInputFooter className="flex-wrap items-center gap-2">
          <PromptInputTools className="flex-wrap gap-1.5">
            <PromptInputActionMenu>
              <PromptInputActionMenuTrigger className="transition-colors duration-200 motion-reduce:transition-none" />
              <PromptInputActionMenuContent>
                <PromptInputActionAddAttachments />
              </PromptInputActionMenuContent>
            </PromptInputActionMenu>
            <PromptInputButton className="transition-colors duration-200 motion-reduce:transition-none">
              <SearchIcon size={16} />
              <span>Search</span>
            </PromptInputButton>
            <ModelSelector
              onOpenChange={setModelSelectorOpen}
              open={modelSelectorOpen}
            >
              <ModelSelectorTrigger asChild>
                <PromptInputButton className="max-w-full transition-colors duration-200 motion-reduce:transition-none">
                  {selectedModelData?.chefSlug && (
                    <ModelSelectorLogo provider={selectedModelData.chefSlug} />
                  )}
                  {selectedModelData?.name && (
                    <ModelSelectorName className="max-w-28">{selectedModelData.name}</ModelSelectorName>
                  )}
                </PromptInputButton>
              </ModelSelectorTrigger>
              <ModelSelectorContent>
                <ModelSelectorInput placeholder="Search models..." />
                <ModelSelectorList>
                  <ModelSelectorEmpty>No models found.</ModelSelectorEmpty>
                  {["OpenAI", "Anthropic", "Google"].map((chef) => (
                    <ModelSelectorGroup heading={chef} key={chef}>
                      {models
                        .filter((m) => m.chef === chef)
                        .map((m) => (
                          <ModelItem
                            key={m.id}
                            m={m}
                            onSelect={handleModelSelect}
                            selectedModel={model}
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
            status={status}
          />
        </PromptInputFooter>
      </PromptInput>
    </PromptInputProvider>
  );
}
