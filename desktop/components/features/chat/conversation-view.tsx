"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import type { ChatMessage, AgentTurnVM } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from "@/components/ai-elements/conversation";
import { Message, MessageResponse } from "@/components/ai-elements/message";
import { BotIcon } from "lucide-react";
import { AgentTurnCard } from "./agent-turn-card";
import { Fragment, useMemo } from "react";
import { TurnCheckpoint } from "./turn-checkpoint";

interface ConversationViewProps {
  sessionId: string;
}

const EMPTY_MESSAGES: ChatMessage[] = [];
const EMPTY_TURNS: AgentTurnVM[] = [];

export function ConversationView({ sessionId }: ConversationViewProps) {
  const messages = useChatStore(
    (state) => state.messages[sessionId] ?? EMPTY_MESSAGES
  );
  const turns = useChatStore((state) => state.turns[sessionId] ?? EMPTY_TURNS);

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
      <ConversationContent className="mx-auto flex max-w-3xl gap-4 px-4 pb-8 pt-4">
        {timeline.length === 0 ? (
          <ConversationEmptyState
            description="发送消息开始对话"
            icon={<BotIcon className="size-12" />}
            title="暂无消息"
          />
        ) : (
          timeline.map((item) =>
            item.kind === "message" ? (
              <Message
                className={
                  item.message.role === "user" ? "ml-0 justify-start" : undefined
                }
                from={item.message.role as "user" | "assistant" | "system"}
                key={item.message.id}
              >
                {item.message.role === "assistant" ? (
                  <MessageResponse className="llm-chat-markdown text-[13px] leading-5 [&_li]:my-0.5 [&_ol]:my-1 [&_p]:my-1 [&_ul]:my-1">
                    {item.message.content}
                  </MessageResponse>
                ) : (
                  <div
                    className={cn(
                      "whitespace-pre-wrap text-[13px] leading-5",
                      item.message.role === "user" &&
                        "w-fit max-w-full rounded-lg bg-secondary/70 px-3 py-2 text-foreground"
                    )}
                  >
                    {item.message.content}
                  </div>
                )}
              </Message>
            ) : (
              <Fragment key={item.turn.id}>
                <AgentTurnCard sessionId={sessionId} turn={item.turn} />
                <TurnCheckpoint sessionId={sessionId} turn={item.turn} />
              </Fragment>
            )
          )
        )}
      </ConversationContent>
    </Conversation>
  );
}
