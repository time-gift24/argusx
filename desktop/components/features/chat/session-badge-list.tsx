"use client";

import { PlusIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useChatStore } from "@/lib/stores/chat-store";
import { SessionBadge } from "./session-badge";
import { BadgeContextMenu } from "./badge-context-menu";
import { cn } from "@/lib/utils";

export function SessionBadgeList() {
  const {
    sessions,
    currentSessionId,
    createSession,
    setCurrentSession,
    updateSession,
    deleteSession,
  } = useChatStore();

  return (
    <div className="flex items-center gap-2 overflow-x-auto px-4 py-2 scrollbar-hide">
      {sessions.map((session) => (
        <BadgeContextMenu
          key={session.id}
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

      {/* 新建会话按钮 */}
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
  );
}
