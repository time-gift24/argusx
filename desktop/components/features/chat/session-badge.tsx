"use client";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { ChatStatus, ChatSession } from "@/lib/stores/chat-store";
import {
  Loader2Icon,
  MessageSquareIcon,
  WrenchIcon,
  TypeIcon,
} from "lucide-react";

interface SessionBadgeProps {
  session: ChatSession;
  isActive: boolean;
  onClick: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
}

const statusConfig: Record<
  ChatStatus,
  { icon: React.ReactNode; label: string }
> = {
  "wait-input": { icon: <MessageSquareIcon className="size-3" />, label: "Ready" },
  thinking: { icon: <Loader2Icon className="size-3 animate-spin" />, label: "Thinking" },
  "tool-call": { icon: <WrenchIcon className="size-3" />, label: "Tool" },
  outputing: { icon: <TypeIcon className="size-3" />, label: "Writing" },
};

export function SessionBadge({
  session,
  isActive,
  onClick,
  onContextMenu,
}: SessionBadgeProps) {
  const status = statusConfig[session.status];

  return (
    <Badge
      className={cn(
        "relative cursor-pointer px-3 py-1.5 transition-all",
        "hover:bg-accent/80",
        isActive && "bg-primary text-primary-foreground hover:bg-primary/90",
        session.color && `bg-${session.color}/20 text-${session.color}-foreground border-${session.color}/30`,
        isActive && `bg-primary`
      )}
      onClick={onClick}
      {...(onContextMenu && { onContextMenu })}
      variant="outline"
    >
      {/* 状态图标 */}
      <span className="mr-1.5">{status.icon}</span>

      {/* 标题 */}
      <span className="max-w-24 truncate text-xs font-medium">
        {session.title}
      </span>
    </Badge>
  );
}
