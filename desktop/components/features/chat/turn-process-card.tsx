"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import { ChevronDownIcon } from "lucide-react";
import { useMemo } from "react";

import { TurnProcessSections } from "./turn-process-sections";
import { buildTurnProcessVM } from "./turn-process-view-model";

interface TurnProcessCardProps {
  sessionId: string;
  turn: AgentTurnVM;
}

const statusDotClass: Record<
  ReturnType<typeof buildTurnProcessVM>["status"],
  string
> = {
  thinking: "bg-amber-500",
  "tool-call": "bg-blue-500",
  outputing: "bg-indigo-500",
  done: "bg-emerald-500",
  failed: "bg-destructive",
};

export function TurnProcessCard({ sessionId, turn }: TurnProcessCardProps) {
  const vm = useMemo(() => buildTurnProcessVM(turn), [turn]);
  const turnUiState = useChatStore(
    (state) => state.turnUiState[sessionId]?.[turn.id]
  );
  const setTurnProcessExpanded = useChatStore(
    (state) => state.setTurnProcessExpanded
  );

  if (!vm.hasProcess) {
    return null;
  }

  const processExpanded = turnUiState?.processExpanded ?? false;

  return (
    <Collapsible
      className="overflow-hidden rounded-md border border-border/60 bg-background/40"
      onOpenChange={(nextOpen) =>
        setTurnProcessExpanded(sessionId, turn.id, nextOpen)
      }
      open={processExpanded}
    >
      <CollapsibleTrigger asChild>
        <button
          className="flex h-8 w-full items-center gap-2 px-2.5 py-1.5 text-left"
          type="button"
        >
          <span
            aria-hidden
            className={cn("size-2 shrink-0 rounded-full", statusDotClass[vm.status])}
          />
          <span className="shrink-0 font-medium text-xs">Process</span>
          <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
            {vm.summary}
          </span>
          <ChevronDownIcon
            className={cn(
              "size-3.5 shrink-0 text-muted-foreground transition-transform",
              processExpanded && "rotate-180"
            )}
          />
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent className="border-border/50 border-t px-2 py-2">
        <TurnProcessSections sessionId={sessionId} turn={turn} vm={vm} />
      </CollapsibleContent>
    </Collapsible>
  );
}
