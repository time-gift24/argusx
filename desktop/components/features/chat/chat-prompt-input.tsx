"use client";

import { ChatStatus } from "@/types";
import { useState } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import {
  PromptInput,
  PromptInputTextarea,
  PromptInputSubmit,
  PromptInputTools,
  PromptInputActionMenu,
  PromptInputActionMenuTrigger,
  PromptInputActionMenuContent,
  PromptInputActionAddAttachments,
} from "@/components/ai-elements/prompt-input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

const MOCK_MODELS = [
  { id: "glm-4", name: "GLM-4" },
  { id: "glm-5", name: "GLM-5" },
  { id: "gpt-4o", name: "GPT-4o" },
];

export function ChatPromptInput() {
  const { currentSessionId, addMessage, updateSessionStatus } = useChatStore();
  const [selectedModel, setSelectedModel] = useState("glm-4");
  const [status, setStatus] = useState<ChatStatus>("ready");

  const handleSubmit = async (message: { text: string }) => {
    if (!currentSessionId || !message.text.trim()) return;

    // 添加用户消息
    addMessage(currentSessionId, {
      role: "user",
      content: message.text,
    });

    // 模拟 AI 响应
    setStatus("submitted");
    updateSessionStatus(currentSessionId, "thinking");

    // Mock: 模拟延迟后添加响应
    setTimeout(() => {
      addMessage(currentSessionId, {
        role: "assistant",
        content: `This is a mock response to: \"${message.text}\"\n\nIn a real implementation, this would be streamed from the LLM backend using Tauri IPC.`,
      });
      updateSessionStatus(currentSessionId, "wait-input");
      setStatus("ready");
    }, 1000);
  };

  return (
    <div className={cn("rounded-xl border border-border bg-card p-2 shadow-lg")}>
      <PromptInput onSubmit={handleSubmit}>
        <PromptInputTextarea
          className="min-h-[60px]"
          placeholder="Send a message..."
        />

        <div className="flex items-center justify-between gap-2 pt-2">
          <PromptInputTools>
            <PromptInputActionMenu>
              <PromptInputActionMenuTrigger />
              <PromptInputActionMenuContent>
                <PromptInputActionAddAttachments />
              </PromptInputActionMenuContent>
            </PromptInputActionMenu>

            {/* 模型选择 */}
            <Select value={selectedModel} onValueChange={setSelectedModel}>
              <SelectTrigger className="h-8 w-28 border-none bg-transparent text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {MOCK_MODELS.map((model) => (
                  <SelectItem key={model.id} value={model.id}>
                    {model.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </PromptInputTools>

          <PromptInputSubmit status={status === "submitted" ? "submitted" : "ready"} />
        </div>
      </PromptInput>
    </div>
  );
}
