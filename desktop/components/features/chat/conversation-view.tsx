"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from "@/components/ai-elements/conversation";
import { Message, MessageResponse } from "@/components/ai-elements/message";
import { BotIcon } from "lucide-react";

interface ConversationViewProps {
  sessionId: string;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  const messages = useChatStore((state) => state.messages[sessionId] ?? []);

  return (
    <Conversation className="h-full min-h-0">
      <ConversationContent className="mx-auto max-w-3xl px-4 pb-8 pt-4">
        {messages.length === 0 ? (
          <ConversationEmptyState
            description="Send a message to start the conversation"
            icon={<BotIcon className="size-12" />}
            title="No messages yet"
          />
        ) : (
          messages.map((message) => (
            <Message key={message.id} from={message.role as "user" | "assistant" | "system"}>
              {message.role === "assistant" ? (
                <MessageResponse>{message.content}</MessageResponse>
              ) : (
                <div className="whitespace-pre-wrap">{message.content}</div>
              )}
            </Message>
          ))
        )}
      </ConversationContent>
    </Conversation>
  );
}
