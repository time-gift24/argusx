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
    <div className="relative flex h-full flex-col">
      {/* 主内容区域 - 消息列表 */}
      <div className="flex-1 overflow-hidden pb-40">
        {currentSession ? (
          <ConversationView sessionId={currentSession.id} />
        ) : (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            Select or create a chat session
          </div>
        )}
      </div>

      {/* 悬浮底部区域 */}
      <div
        className="fixed bottom-0 left-0 right-0 z-50 bg-background/80 backdrop-blur-xl"
      >
        {/* Badge 列表 */}
        <SessionBadgeList />

        {/* 输入框 */}
        <div className="mx-auto max-w-3xl p-4 pt-0">
          <ChatPromptInput />
        </div>
      </div>
    </div>
  );
}
