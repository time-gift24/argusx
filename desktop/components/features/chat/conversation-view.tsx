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
import { useStickToBottomContext } from "use-stick-to-bottom";
import { AgentTurnCard } from "./agent-turn-card";
import { sanitizeAssistantMarkdown } from "./sanitize-assistant-markdown";
import { Fragment, useEffect, useMemo } from "react";
import { TurnCheckpoint } from "./turn-checkpoint";

interface ConversationViewProps {
  sessionId: string;
}

const EMPTY_MESSAGES: ChatMessage[] = [];
const EMPTY_TURNS: AgentTurnVM[] = [];

interface ConversationScrollSyncProps {
  signal: number;
}

function ConversationScrollSync({ signal }: ConversationScrollSyncProps) {
  const { scrollToBottom } = useStickToBottomContext();

  useEffect(() => {
    if (signal <= 0) {
      return;
    }

    let raf1 = 0;
    let raf2 = 0;
    raf1 = window.requestAnimationFrame(() => {
      scrollToBottom();
      raf2 = window.requestAnimationFrame(() => {
        scrollToBottom();
      });
    });

    return () => {
      if (raf1) {
        window.cancelAnimationFrame(raf1);
      }
      if (raf2) {
        window.cancelAnimationFrame(raf2);
      }
    };
  }, [signal, scrollToBottom]);

  return null;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  const messages = useChatStore(
    (state) => state.messages[sessionId] ?? EMPTY_MESSAGES
  );
  const turns = useChatStore((state) => state.turns[sessionId] ?? EMPTY_TURNS);
  const scrollSignal = useChatStore(
    (state) => state.scrollToBottomSignal[sessionId] ?? 0
  );

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
      <ConversationScrollSync signal={scrollSignal} />
      <ConversationContent className="mx-auto flex max-w-3xl gap-4 px-4 pb-8 pt-4">
        {timeline.length === 0 ? (
          <ConversationEmptyState
            description="发送消息开始对话"
            icon={<BotIcon className="size-12" />}
            title="暂无消息"
          />
        ) : (
          timeline.map((item) => {
            if (item.kind === "message") {
              const assistantContent =
                item.message.role === "assistant"
                  ? sanitizeAssistantMarkdown(item.message.content)
                  : item.message.content;

              if (
                item.message.role === "assistant" &&
                assistantContent.length === 0
              ) {
                return null;
              }

              return (
                <Message
                  className={
                    item.message.role === "user" ? "ml-0 justify-start" : undefined
                  }
                  from={item.message.role as "user" | "assistant" | "system"}
                  key={item.message.id}
                >
                  {item.message.role === "assistant" ? (
                    <MessageResponse className="llm-chat-markdown text-[13px] leading-5 [&_li]:my-0.5 [&_ol]:my-1 [&_p]:my-1 [&_ul]:my-1">
                      {assistantContent}
                    </MessageResponse>
                  ) : (
                    <div
                      className={cn(
                        "whitespace-pre-wrap text-[13px] leading-5",
                        item.message.role === "user" &&
                          "w-fit max-w-full rounded-lg bg-secondary/70 px-3 py-2 text-foreground"
                      )}
                    >
                      {assistantContent}
                    </div>
                  )}
                </Message>
              );
            }

            return (
              <Fragment key={item.turn.id}>
                <AgentTurnCard sessionId={sessionId} turn={item.turn} />
                <TurnCheckpoint sessionId={sessionId} turn={item.turn} />
              </Fragment>
            );
          })
        )}
      </ConversationContent>
    </Conversation>
  );
}
