"use client";

import { useEffect } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import { ConversationView } from "./conversation-view";
import { ChatSessionBar } from "./chat-session-bar";

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

      {/* Floating bottom area - session bar with badges and input */}
      <ChatSessionBar />
    </div>
  );
}
