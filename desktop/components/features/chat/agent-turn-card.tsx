"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import { Message, MessageResponse } from "@/components/ai/message";

import { CHAT_MARKDOWN_CLASS } from "./chat-markdown-class";
import { TurnProcessCard } from "./turn-process-card";
import { sanitizeAssistantMarkdown } from "./sanitize-assistant-markdown";

interface AgentTurnCardProps {
  sessionId: string;
  turn: AgentTurnVM;
}

export function AgentTurnCard({ sessionId, turn }: AgentTurnCardProps) {
  const summaryText = sanitizeAssistantMarkdown(turn.assistantText);
  const shouldShowSummary = summaryText.length > 0;

  return (
    <Message from="assistant">
      <div className="w-full space-y-2">
        {shouldShowSummary && (
          <MessageResponse className={CHAT_MARKDOWN_CLASS}>
            {summaryText}
          </MessageResponse>
        )}

        <TurnProcessCard sessionId={sessionId} turn={turn} />

        {turn.status === "failed" || turn.status === "cancelled" ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-sm">
            {turn.error ?? "轮次失败"}
          </div>
        ) : null}
      </div>
    </Message>
  );
}
