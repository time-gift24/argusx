import type { ChatMessage } from "@/lib/stores/chat-store";

interface ChatMessagesSlice {
  messages: Record<string, ChatMessage[] | undefined>;
}

export const EMPTY_CHAT_MESSAGES: ChatMessage[] = [];

export const selectSessionMessages =
  (sessionId: string | null) =>
  (state: ChatMessagesSlice): ChatMessage[] => {
    if (!sessionId) {
      return EMPTY_CHAT_MESSAGES;
    }
    return state.messages[sessionId] ?? EMPTY_CHAT_MESSAGES;
  };
