"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import type { ChatMessage, AgentTurnVM } from "@/lib/stores/chat-store";
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from "@/components/ai-elements/conversation";
import { Message, MessageResponse } from "@/components/ai-elements/message";
import { BotIcon } from "lucide-react";
import { AgentTurnCard } from "./agent-turn-card";
import { useMemo } from "react";

interface ConversationViewProps {
  sessionId: string;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  // Extract messages and turns with proper memoization to avoid infinite loop
  const messages = useChatStore((state) => state.messages[sessionId] ?? []);
  const turns = useChatStore((state) => state.turns[sessionId] ?? []);

  const timeline: Array<
    | { kind: "message"; at: number; message: ChatMessage }
    | { kind: "turn"; at: number; turn: AgentTurnVM }
  > = useMemo(() => {
    const items: Array<
      | { kind: "message"; at: number; message: ChatMessage }
      | { kind: "turn"; at: number; turn: AgentTurnVM }
    > = [
      ...messages.map((message) => ({
        kind: "message" as const,
        at: message.createdAt,
        message,
      })),
      ...turns.map((turn) => ({
        kind: "turn" as const,
        at: turn.createdAt,
        turn,
      })),
    ];
    return items.sort((a, b) => a.at - b.at);
  }, [messages, turns]);

  return (
    <Conversation className="h-full min-h-0">
      <ConversationContent className="mx-auto max-w-3xl px-4 pb-8 pt-4">
        {timeline.length === 0 ? (
          <ConversationEmptyState
            description="Send a message to start the conversation"
            icon={<BotIcon className="size-12" />}
            title="No messages yet"
          />
        ) : (
          timeline.map((item) =>
            item.kind === "message" ? (
              <Message
                from={item.message.role as "user" | "assistant" | "system"}
                key={item.message.id}
              >
                {item.message.role === "assistant" ? (
                  <MessageResponse>{item.message.content}</MessageResponse>
                ) : (
                  <div className="whitespace-pre-wrap">{item.message.content}</div>
                )}
              </Message>
            ) : (
              <AgentTurnCard
                key={item.turn.id}
                sessionId={sessionId}
                turn={item.turn}
              />
            )
          )
        )}
      </ConversationContent>
    </Conversation>
  );
}
