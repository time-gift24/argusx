"use client";

import { PlusIcon } from "lucide-react";
import dynamic from "next/dynamic";
import { Button } from "@/components/ui/button";
import { useChatStore } from "@/lib/stores/chat-store";
import { SessionBadge } from "./session-badge";
import { BadgeContextMenu } from "./badge-context-menu";
import { cn } from "@/lib/utils";
import { useEffect, useRef } from "react";

const ChatPromptInput = dynamic(
  () => import("./chat-prompt-input").then((module) => module.ChatPromptInput),
  {
    loading: () => (
      <div
        aria-hidden
        className="h-14 w-full rounded-2xl border border-border/40 bg-background/50 backdrop-blur-xl"
      />
    ),
    ssr: false,
  }
);

interface ChatSessionBarProps {
  onHeightChange?: (height: number) => void;
}

export function ChatSessionBar({ onHeightChange }: ChatSessionBarProps) {
  const {
    sessions,
    currentSessionId,
    createSession,
    setCurrentSession,
    updateSession,
    deleteSession,
  } = useChatStore();
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!onHeightChange || !containerRef.current) {
      return;
    }

    const element = containerRef.current;
    const notifyHeight = () => {
      const nextHeight = Math.ceil(element.getBoundingClientRect().height);
      onHeightChange(nextHeight);
    };

    notifyHeight();

    if (typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(notifyHeight);
    observer.observe(element);

    return () => observer.disconnect();
  }, [onHeightChange]);

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-0 z-40" ref={containerRef}>
      {/* Single container with badges and input - left-aligned */}
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-2 px-4 pb-[calc(env(safe-area-inset-bottom)+0.75rem)] pt-0">
        {/* Badge list - left-aligned */}
        <div className="pointer-events-auto flex items-center gap-2 overflow-x-auto px-2.5 py-2 scrollbar-hide">
          {sessions.map((session) => (
            <BadgeContextMenu
              key={session.id}
              onChangeColor={(color) => updateSession(session.id, { color })}
              onDelete={() => deleteSession(session.id)}
              onRename={(title) => updateSession(session.id, { title })}
              session={session}
            >
              <SessionBadge
                isActive={session.id === currentSessionId}
                onClick={() => setCurrentSession(session.id)}
                session={session}
              />
            </BadgeContextMenu>
          ))}

          {/* New session button */}
          <Button
            className={cn(
              "shrink-0 rounded-full",
              "border border-dashed border-muted-foreground/50",
              "transition-colors duration-200 motion-reduce:transition-none",
              "hover:border-primary hover:bg-primary/10"
            )}
            onClick={() => createSession()}
            size="icon"
            variant="ghost"
          >
            <PlusIcon className="size-4" />
          </Button>
        </div>

        {/* Input box */}
        <div className="pointer-events-auto">
          <ChatPromptInput />
        </div>
      </div>
    </div>
  );
}
