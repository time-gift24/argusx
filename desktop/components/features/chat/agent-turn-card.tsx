"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import { Message, MessageResponse } from "@/components/ai-elements/message";
import { Loader2Icon } from "lucide-react";

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

        {turn.status === "streaming" &&
          turn.postValidationAttempt !== undefined &&
          turn.postValidationMaxAttempts !== undefined && (
            <div className="flex items-center gap-2 rounded-lg border border-border/80 bg-background/70 px-3 py-2 text-muted-foreground text-sm">
              <Loader2Icon className="size-3.5 animate-spin" />
              <span>
                Validating output (attempt {turn.postValidationAttempt + 1}/
                {turn.postValidationMaxAttempts})
              </span>
            </div>
          )}

        {turn.status === "failed" || turn.status === "cancelled" ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-sm">
            {turn.error ?? "Turn failed"}
          </div>
        ) : null}
      </div>
    </Message>
  );
}
