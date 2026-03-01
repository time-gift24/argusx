"use client";

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { MessageList } from "./message-list";
import { ChatInput } from "./chat-input";
import { SessionSwitcher } from "./session-switcher";
import { StatusBar } from "./status-bar";
import { useChatStore } from "@/lib/stores/chat-store";

export function ChatPage() {
  const {
    updateAssistantMessage,
    setAgentStatus,
    setReasoningText,
    addToolCall,
    updateToolCall,
    loadSessions,
    createSession
  } = useChatStore();

  useEffect(() => {
    // 初始化：加载 sessions，如果没有则创建
    const init = async () => {
      await loadSessions();
      const { sessions } = useChatStore.getState();
      if (sessions.length === 0) {
        await createSession("New Chat");
      }
    };
    init();

    // 监听流式事件
    const unlisten = listen<{
      event_type: string;
      data: Record<string, string>;
    }>("chat-stream-event", (event) => {
      const { event_type, data } = event.payload;
      switch (event_type) {
        case "message_delta":
          updateAssistantMessage(data.content || "");
          break;
        case "reasoning":
          setReasoningText(data.content || "");
          break;
        case "tool_start":
          addToolCall({
            callId: data.call_id,
            toolName: data.tool_name,
            status: "running"
          });
          break;
        case "tool_end":
          updateToolCall(data.call_id, { status: "done", output: data.output });
          break;
        case "turn_done":
          setAgentStatus("idle");
          setReasoningText("");
          break;
        case "error":
          setAgentStatus("error");
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex flex-col h-screen">
      <MessageList />
      <StatusBar />
      <SessionSwitcher />
      <ChatInput />
    </div>
  );
}
