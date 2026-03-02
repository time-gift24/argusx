"use client";

import { useCallback, useEffect, useState } from "react";
import { listenAgentStream } from "@/lib/api/chat";
import { CHAT_SIDEBAR_MIN_WIDTH } from "@/lib/layout/chat-layout";
import { useChatStore } from "@/lib/stores/chat-store";
import { useLlmRuntimeConfigStore } from "@/lib/stores/llm-runtime-config-store";
import { Button } from "@/components/ui/button";
import { Settings2Icon } from "lucide-react";
import { ConversationView } from "./conversation-view";
import { ChatSessionBar } from "./chat-session-bar";
import { ChatRuntimeConfigDialog } from "./chat-runtime-config-dialog";

export function ChatPage() {
  const { sessions, currentSessionId, createSession } = useChatStore();
  const bootstrapLlmConfig = useLlmRuntimeConfigStore((state) => state.bootstrap);
  const [composerHeight, setComposerHeight] = useState(180);
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [configDialogSeed, setConfigDialogSeed] = useState(0);

  // 如果没有会话，自动创建一个
  useEffect(() => {
    if (sessions.length === 0) {
      createSession();
    }
  }, [sessions.length, createSession]);

  useEffect(() => {
    void bootstrapLlmConfig();
  }, [bootstrapLlmConfig]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listenAgentStream((envelope) => {
      useChatStore.getState().applyAgentStreamEnvelope(envelope);
    })
      .then((cleanup) => {
        if (disposed) {
          cleanup();
          return;
        }
        unlisten = cleanup;
      })
      .catch((error) => {
        console.error("Failed to listen agent stream", error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const currentSession = sessions.find((s) => s.id === currentSessionId);
  const handleHeightChange = useCallback((height: number) => {
    setComposerHeight((current) => (current === height ? current : height));
  }, []);
  const handleOpenConfigDialog = useCallback(() => {
    setConfigDialogSeed((value) => value + 1);
    setConfigDialogOpen(true);
  }, []);

  return (
    <div
      className="relative flex min-h-0 flex-1 flex-col"
      style={{ minWidth: `${CHAT_SIDEBAR_MIN_WIDTH}px` }}
    >
      <div className="absolute right-3 top-3 z-50">
        <div className="relative">
          <span
            aria-hidden
            className="pointer-events-none absolute inset-0 rounded-full bg-primary/30 animate-ping motion-reduce:animate-none"
          />
          <Button
            className="relative border-primary/40 bg-background/90 shadow-sm hover:bg-background"
            onClick={handleOpenConfigDialog}
            size="icon-sm"
            type="button"
            variant="outline"
          >
            <Settings2Icon className="size-4" />
            <span className="sr-only">Open runtime config</span>
          </Button>
        </div>
      </div>
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
      {/* Floating bottom area - session bar with badges and input */}
      <ChatSessionBar onHeightChange={handleHeightChange} />
      <ChatRuntimeConfigDialog
        key={configDialogSeed}
        onOpenChange={setConfigDialogOpen}
        open={configDialogOpen}
      />
    </div>
  );
}
