"use client";

import { PlusIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useChatStore } from "@/lib/stores/chat-store";
import { SessionBadge } from "./session-badge";
import { BadgeContextMenu } from "./badge-context-menu";
import { ChatPromptInput } from "./chat-prompt-input";
import { cn } from "@/lib/utils";

export function ChatSessionBar() {
  const {
    sessions,
    currentSessionId,
    createSession,
    setCurrentSession,
    updateSession,
    deleteSession,
  } = useChatStore();

  return (
    <div className="sticky bottom-0 z-50 bg-background/80 backdrop-blur-xl">
      {/* Single container with badges and input - left-aligned */}
      <div className="mx-auto flex max-w-3xl flex-col gap-2 p-4 pt-0">
        {/* Badge list - left-aligned */}
        <div className="flex items-center gap-2 overflow-x-auto px-2 py-2 scrollbar-hide">
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
        <ChatPromptInput />
      </div>
    </div>
  );
}
