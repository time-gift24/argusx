"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import { Message, MessageResponse } from "@/components/ai-elements/message";

import { TurnProcessCard } from "./turn-process-card";

interface AgentTurnCardProps {
  sessionId: string;
  turn: AgentTurnVM;
}

export function AgentTurnCard({ sessionId, turn }: AgentTurnCardProps) {
  const summaryText = turn.assistantText.trim();
  const shouldShowSummary = summaryText.length > 0;

  return (
    <Message from="assistant">
      <div className="w-full space-y-2">
        {shouldShowSummary && (
          <MessageResponse className="text-[13px] leading-5 [&_li]:my-0.5 [&_ol]:my-1 [&_p]:my-1 [&_ul]:my-1">
            {summaryText}
          </MessageResponse>
        )}

        <TurnProcessCard sessionId={sessionId} turn={turn} />

        {turn.status === "failed" || turn.status === "cancelled" ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-sm">
            {turn.error ?? "Turn failed"}
          </div>
        ) : null}
      </div>
    </Message>
  );
}
