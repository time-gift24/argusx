"use client";

import { useCallback, useEffect, useState } from "react";
import { listenAgentStream } from "@/lib/api/chat";
import { CHAT_SIDEBAR_MIN_WIDTH } from "@/lib/layout/chat-layout";
import { useChatStore } from "@/lib/stores/chat-store";
import { useLlmRuntimeConfigStore } from "@/lib/stores/llm-runtime-config-store";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { HistoryIcon, Settings2Icon } from "lucide-react";
import { ConversationView } from "./conversation-view";
import { ChatSessionBar } from "./chat-session-bar";
import { ChatRuntimeConfigDialog } from "./chat-runtime-config-dialog";
import { ChatFullInfoDialog } from "./chat-full-info-dialog";

export function ChatPage() {
  const { sessions, currentSessionId, bootstrap } = useChatStore();
  const bootstrapLlmConfig = useLlmRuntimeConfigStore((state) => state.bootstrap);
  const availableModels = useLlmRuntimeConfigStore((state) => state.availableModels);
  const isConfigLoading = useLlmRuntimeConfigStore((state) => state.loading);
  const [composerHeight, setComposerHeight] = useState(180);
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [configDialogSeed, setConfigDialogSeed] = useState(0);
  const hasAvailableModels = availableModels.length > 0;
  const showNoModelHint = !isConfigLoading && !hasAvailableModels;
  const [fullInfoDialogOpen, setFullInfoDialogOpen] = useState(false);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

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
  const handleOpenFullInfoDialog = useCallback(() => {
    setFullInfoDialogOpen(true);
  }, []);

  return (
    <div
      className="relative flex min-h-0 flex-1 flex-col"
      style={{ minWidth: `${CHAT_SIDEBAR_MIN_WIDTH}px` }}
    >
      <div className="absolute left-3 top-3 z-50">
        <Button
          className="border-primary/30 bg-background/90 shadow-sm hover:bg-background"
          onClick={handleOpenFullInfoDialog}
          size="sm"
          type="button"
          variant="outline"
        >
          <HistoryIcon className="mr-1 size-4" />
          完整信息
        </Button>
      </div>
      <div className="absolute right-3 top-3 z-50">
        <div className="relative">
          <span
            aria-hidden
            className={cn(
              "pointer-events-none absolute inset-0 rounded-full motion-reduce:animate-none",
              showNoModelHint ? "bg-orange-500/30 animate-ping" : "bg-primary/30"
            )}
          />
          <Button
            className={cn(
              "relative border bg-background/90 shadow-sm hover:bg-background",
              showNoModelHint
                ? "border-orange-500/40 text-orange-600 hover:text-orange-700 dark:text-orange-400 dark:hover:text-orange-300"
                : "border-primary/40 text-primary hover:text-primary"
            )}
            onClick={handleOpenConfigDialog}
            size="icon-sm"
            type="button"
            variant="outline"
          >
            <Settings2Icon className="size-4" />
            <span className="sr-only">打开运行时配置</span>
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
            选择或创建对话会话
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
      <ChatFullInfoDialog
        onOpenChange={setFullInfoDialogOpen}
        open={fullInfoDialogOpen}
        sessionId={currentSessionId}
      />
    </div>
  );
}
