"use client";

import type { AgentTurnVM, QueueItemVM } from "@/lib/stores/chat-store";

import {
  Plan,
  PlanAction,
  PlanContent,
  PlanDescription,
  PlanHeader,
  PlanTitle,
  PlanTrigger,
} from "@/components/ai-elements/plan";
import {
  Queue,
  QueueItem,
  QueueItemContent,
  QueueItemIndicator,
  QueueList,
} from "@/components/ai-elements/queue";
import { TaskItem } from "@/components/ai-elements/task";
import { Terminal } from "@/components/ai-elements/terminal";
import {
  Tool,
  ToolContent,
  ToolHeader,
  ToolInput,
  ToolOutput,
} from "@/components/ai-elements/tool";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import {
  ChevronDownIcon,
  LightbulbIcon,
  ListTodoIcon,
  Loader2Icon,
  TerminalIcon,
  WrenchIcon,
} from "lucide-react";

import type {
  TurnProcessSectionKey,
  TurnProcessSectionVM,
  TurnProcessVM,
} from "./turn-process-view-model";

interface TurnProcessSectionsProps {
  sessionId: string;
  turn: AgentTurnVM;
  vm: TurnProcessVM;
}

const queueStatusLabel: Record<QueueItemVM["status"], string> = {
  waiting: "Waiting",
  running: "Running",
  completed: "Completed",
  failed: "Failed",
};

const getSectionIcon = (key: TurnProcessSectionKey) => {
  if (key === "reasoning") {
    return LightbulbIcon;
  }
  if (key === "plan") {
    return ListTodoIcon;
  }
  if (key === "tools") {
    return WrenchIcon;
  }
  return TerminalIcon;
};

const isToolStreaming = (state: AgentTurnVM["tools"][number]["state"]) =>
  state === "input-available" || state === "input-streaming";

export function TurnProcessSections({
  sessionId,
  turn,
  vm,
}: TurnProcessSectionsProps) {
  const turnUiState = useChatStore(
    (state) => state.turnUiState[sessionId]?.[turn.id]
  );
  const setTurnSectionExpanded = useChatStore(
    (state) => state.setTurnSectionExpanded
  );
  const setTurnCodeExpanded = useChatStore((state) => state.setTurnCodeExpanded);

  const isSectionOpen = (section: TurnProcessSectionVM): boolean =>
    turnUiState?.sectionExpanded[section.key] ?? section.defaultOpen;

  const renderReasoning = (section: TurnProcessSectionVM) => {
    const Icon = getSectionIcon(section.key);
    const open = isSectionOpen(section);

    return (
      <Collapsible
        className="overflow-hidden rounded-md border border-border/60 bg-background/40"
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={open}
      >
        <CollapsibleTrigger asChild>
          <button
            className="flex h-8 w-full items-center gap-2 px-2.5 py-1.5 text-left"
            type="button"
          >
            <Icon className="size-3.5 text-muted-foreground" />
            <span className="shrink-0 font-medium text-xs">Reasoning</span>
            {section.isStreaming && (
              <Loader2Icon className="size-3.5 animate-spin text-muted-foreground" />
            )}
            <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
              {section.preview}
            </span>
            <ChevronDownIcon
              className={cn(
                "size-3.5 text-muted-foreground transition-transform",
                open && "rotate-180"
              )}
            />
          </button>
        </CollapsibleTrigger>
        <CollapsibleContent className="border-border/50 border-t px-2.5 py-2">
          <p className="whitespace-pre-wrap text-[12px] leading-5 text-foreground/90">
            {turn.reasoning.text || "Streaming reasoning..."}
          </p>
        </CollapsibleContent>
      </Collapsible>
    );
  };

  const renderPlan = (section: TurnProcessSectionVM) => {
    if (!turn.plan) {
      return null;
    }

    return (
      <Plan
        compact
        isStreaming={turn.plan.isStreaming}
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isSectionOpen(section)}
      >
        <PlanHeader>
          <div className="min-w-0 space-y-0.5">
            <PlanTitle className="truncate">{turn.plan.title}</PlanTitle>
            <PlanDescription className="truncate">{section.preview}</PlanDescription>
          </div>
          <PlanAction>
            <PlanTrigger compact />
          </PlanAction>
        </PlanHeader>
        <PlanContent className="space-y-1.5">
          {turn.plan.tasks.map((task) => (
            <TaskItem compact key={task.id}>
              <span className="inline-flex items-center gap-1.5">
                <span>{task.status === "completed" ? "✓" : "•"}</span>
                <span className={cn(task.status === "completed" && "line-through")}>
                  {task.title}
                </span>
              </span>
            </TaskItem>
          ))}
        </PlanContent>
      </Plan>
    );
  };

  const renderTools = (section: TurnProcessSectionVM) => {
    const Icon = getSectionIcon(section.key);
    const open = isSectionOpen(section);

    return (
      <Collapsible
        className="overflow-hidden rounded-md border border-border/60 bg-background/40"
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={open}
      >
        <CollapsibleTrigger asChild>
          <button
            className="flex h-8 w-full items-center gap-2 px-2.5 py-1.5 text-left"
            type="button"
          >
            <Icon className="size-3.5 text-muted-foreground" />
            <span className="shrink-0 font-medium text-xs">Tools</span>
            <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
              {section.preview}
            </span>
            <ChevronDownIcon
              className={cn(
                "size-3.5 text-muted-foreground transition-transform",
                open && "rotate-180"
              )}
            />
          </button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-1.5 border-border/50 border-t px-2.5 py-2">
          {turn.queue.items.length > 0 && (
            <Queue compact>
              <div className="px-0.5 py-0.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                Queue
              </div>
              <QueueList compact className="mt-0">
                {turn.queue.items.map((item) => (
                  <QueueItem compact key={item.callId}>
                    <div className="flex items-center gap-2">
                      <QueueItemIndicator completed={item.status === "completed"} />
                      <QueueItemContent completed={item.status === "completed"}>
                        {item.toolName} · {queueStatusLabel[item.status]}
                      </QueueItemContent>
                    </div>
                  </QueueItem>
                ))}
              </QueueList>
            </Queue>
          )}

          {turn.tools.map((tool) => {
            const toolOutputKey = `${turn.id}:tool:${tool.callId}`;
            const outputExpanded = Boolean(turnUiState?.codeExpanded[toolOutputKey]);

            return (
              <Tool
                compact
                defaultOpen={tool.state === "output-error" || isToolStreaming(tool.state)}
                key={tool.callId}
              >
                <ToolHeader
                  compact
                  state={tool.state}
                  toolName={tool.toolName}
                  type="dynamic-tool"
                />
                <ToolContent compact>
                  {tool.input && <ToolInput compact input={tool.input} />}
                  <ToolOutput
                    compact
                    errorText={tool.errorText}
                    expanded={outputExpanded}
                    onToggleExpand={() =>
                      setTurnCodeExpanded(
                        sessionId,
                        turn.id,
                        toolOutputKey,
                        !outputExpanded
                      )
                    }
                    output={tool.output}
                    previewLines={8}
                  />
                </ToolContent>
              </Tool>
            );
          })}
        </CollapsibleContent>
      </Collapsible>
    );
  };

  const renderTerminal = (section: TurnProcessSectionVM) => {
    const Icon = getSectionIcon(section.key);
    const open = isSectionOpen(section);
    const terminalOutputKey = `${turn.id}:terminal`;
    const outputExpanded = Boolean(turnUiState?.codeExpanded[terminalOutputKey]);

    return (
      <Collapsible
        className="overflow-hidden rounded-md border border-border/60 bg-background/40"
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={open}
      >
        <CollapsibleTrigger asChild>
          <button
            className="flex h-8 w-full items-center gap-2 px-2.5 py-1.5 text-left"
            type="button"
          >
            <Icon className="size-3.5 text-muted-foreground" />
            <span className="shrink-0 font-medium text-xs">Terminal</span>
            {section.isStreaming && (
              <Loader2Icon className="size-3.5 animate-spin text-muted-foreground" />
            )}
            <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
              {section.preview}
            </span>
            <ChevronDownIcon
              className={cn(
                "size-3.5 text-muted-foreground transition-transform",
                open && "rotate-180"
              )}
            />
          </button>
        </CollapsibleTrigger>
        <CollapsibleContent className="border-border/50 border-t px-2.5 py-2">
          <Terminal
            compact
            expanded={outputExpanded}
            isStreaming={turn.terminal.isStreaming}
            onToggleExpand={(expanded) =>
              setTurnCodeExpanded(
                sessionId,
                turn.id,
                terminalOutputKey,
                expanded
              )
            }
            output={vm.terminalOutput}
            previewLines={8}
          />
        </CollapsibleContent>
      </Collapsible>
    );
  };

  return (
    <div className="space-y-1.5">
      {vm.sections.map((section) => {
        if (section.key === "reasoning") {
          return renderReasoning(section);
        }
        if (section.key === "plan") {
          return renderPlan(section);
        }
        if (section.key === "tools") {
          return renderTools(section);
        }
        return renderTerminal(section);
      })}
    </div>
  );
}
