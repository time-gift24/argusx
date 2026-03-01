"use client";

import { useEffect } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import { ConversationView } from "./conversation-view";
import { SessionBadgeList } from "./session-badge-list";
import { ChatPromptInput } from "./chat-prompt-input";

export function ChatPage() {
  const { sessions, currentSessionId, createSession } = useChatStore();

  // 如果没有会话，自动创建一个
  useEffect(() => {
    if (sessions.length === 0) {
      createSession();
    }
  }, [sessions.length, createSession]);

  const currentSession = sessions.find((s) => s.id === currentSessionId);

  return (
    <div className="flex h-full flex-col">
      {/* 主内容区域 - 消息列表 */}
      <div className="flex-1 overflow-hidden">
        {currentSession ? (
          <ConversationView sessionId={currentSession.id} />
        ) : (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            Select or create a chat session
          </div>
        )}
      </div>

      {/* 底部区域 */}
      <div className="border-t border-border/50 p-4">
        {/* Badge 列表 */}
        <div className="mb-4">
          <SessionBadgeList />
        </div>

        {/* 输入框 */}
        <div className="mx-auto max-w-3xl">
          <ChatPromptInput />
        </div>
      </div>
    </div>
  );
}
