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

/** Status color palette based on session status */
const statusColors: Record<ChatStatus, { bg: string; text: string; dot: string }> = {
  "wait-input": { bg: "bg-muted", text: "text-muted-foreground", dot: "bg-gray-400" },
  thinking: { bg: "bg-blue-500/20", text: "text-blue-600 dark:text-blue-400", dot: "bg-blue-500" },
  "tool-call": { bg: "bg-amber-500/20", text: "text-amber-600 dark:text-amber-400", dot: "bg-amber-500" },
  outputing: { bg: "bg-emerald-500/20", text: "text-emerald-600 dark:text-emerald-400", dot: "bg-emerald-500" },
};

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
  const colors = statusColors[session.status];

  /** Get badge color based on session.color or status */
  const getBadgeColors = () => {
    // If session has custom color, use it
    if (session.color) {
      const colorMap: Record<string, { bg: string; text: string; dot: string }> = {
        red: { bg: "bg-red-500/20", text: "text-red-600 dark:text-red-400", dot: "bg-red-500" },
        orange: { bg: "bg-orange-500/20", text: "text-orange-600 dark:text-orange-400", dot: "bg-orange-500" },
        amber: { bg: "bg-amber-500/20", text: "text-amber-600 dark:text-amber-400", dot: "bg-amber-500" },
        yellow: { bg: "bg-yellow-500/20", text: "text-yellow-600 dark:text-yellow-400", dot: "bg-yellow-500" },
        lime: { bg: "bg-lime-500/20", text: "text-lime-600 dark:text-lime-400", dot: "bg-lime-500" },
        green: { bg: "bg-emerald-500/20", text: "text-emerald-600 dark:text-emerald-400", dot: "bg-emerald-500" },
        emerald: { bg: "bg-emerald-500/20", text: "text-emerald-600 dark:text-emerald-400", dot: "bg-emerald-500" },
        teal: { bg: "bg-teal-500/20", text: "text-teal-600 dark:text-teal-400", dot: "bg-teal-500" },
        cyan: { bg: "bg-cyan-500/20", text: "text-cyan-600 dark:text-cyan-400", dot: "bg-cyan-500" },
        sky: { bg: "bg-sky-500/20", text: "text-sky-600 dark:text-sky-400", dot: "bg-sky-500" },
        blue: { bg: "bg-blue-500/20", text: "text-blue-600 dark:text-blue-400", dot: "bg-blue-500" },
        indigo: { bg: "bg-indigo-500/20", text: "text-indigo-600 dark:text-indigo-400", dot: "bg-indigo-500" },
        violet: { bg: "bg-violet-500/20", text: "text-violet-600 dark:text-violet-400", dot: "bg-violet-500" },
        purple: { bg: "bg-purple-500/20", text: "text-purple-600 dark:text-purple-400", dot: "bg-purple-500" },
        fuchsia: { bg: "bg-fuchsia-500/20", text: "text-fuchsia-600 dark:text-fuchsia-400", dot: "bg-fuchsia-500" },
        pink: { bg: "bg-pink-500/20", text: "text-pink-600 dark:text-pink-400", dot: "bg-pink-500" },
        rose: { bg: "bg-rose-500/20", text: "text-rose-600 dark:text-rose-400", dot: "bg-rose-500" },
      };
      return colorMap[session.color] || colors;
    }
    return colors;
  };

  const badgeColors = getBadgeColors();
  const isActiveStatus = session.status === "thinking" || session.status === "tool-call" || session.status === "outputing";

  return (
    <Badge
      className={cn(
        "relative cursor-pointer rounded-lg px-3 py-1.5 transition-all",
        "hover:opacity-90",
        isActive && "ring-2 ring-primary ring-offset-1 dark:ring-offset-background",
        badgeColors.bg,
        badgeColors.text
      )}
      onClick={onClick}
      {...(onContextMenu && { onContextMenu })}
      variant="outline"
    >
      {/* Status indicator dot - top-left corner */}
      <span
        className={cn(
          "absolute -top-1 -left-1 h-2.5 w-2.5 rounded-full border-2 border-background",
          badgeColors.dot,
          (isActive || isActiveStatus) && "animate-pulse"
        )}
        title={status.label}
      />

      {/* Status icon (smaller, for visual feedback) */}
      <span className="mr-1.5 opacity-70">{status.icon}</span>

      {/* Title */}
      <span className="max-w-24 truncate text-xs font-medium">
        {session.title}
      </span>
    </Badge>
  );
}
