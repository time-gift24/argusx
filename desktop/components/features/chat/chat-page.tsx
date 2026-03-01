"use client";

import { useCallback, useEffect, useState } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import { ConversationView } from "./conversation-view";
import { ChatSessionBar } from "./chat-session-bar";

export function ChatPage() {
  const { sessions, currentSessionId, createSession } = useChatStore();
  const [composerHeight, setComposerHeight] = useState(180);

  // 如果没有会话，自动创建一个
  useEffect(() => {
    if (sessions.length === 0) {
      createSession();
    }
  }, [sessions.length, createSession]);

  const currentSession = sessions.find((s) => s.id === currentSessionId);
  const handleHeightChange = useCallback((height: number) => {
    setComposerHeight((current) => (current === height ? current : height));
  }, []);

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {/* 主内容区域 - 消息列表 */}
      <div
        className="relative flex-1 min-h-0 overflow-hidden"
        style={{ paddingBottom: `${composerHeight + 24}px` }}
      >
        {currentSession ? (
          <ConversationView sessionId={currentSession.id} />
        ) : (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            Select or create a chat session
          </div>
        )}
      </div>
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-4 z-30 h-24 rounded-t-3xl bg-gradient-to-t from-background via-background/85 to-transparent"
        style={{ bottom: `${composerHeight}px` }}
      />

      {/* Floating bottom area - session bar with badges and input */}
      <ChatSessionBar onHeightChange={handleHeightChange} />
    </div>
  );
}
