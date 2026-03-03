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
import {
  Reasoning,
  ReasoningContent,
  ReasoningTrigger,
  useReasoning,
} from "@/components/ai-elements/reasoning";
import { Shimmer } from "@/components/ai-elements/shimmer";
import { TaskItem } from "@/components/ai-elements/task";
import {
  Terminal,
  TerminalContent,
  TerminalCopyButton,
} from "@/components/ai-elements/terminal";
import {
  Tool,
  ToolContent,
  ToolHeader,
  ToolInput,
  ToolOutput,
} from "@/components/ai-elements/tool";
import { Button } from "@/components/ui/button";
import {
  CollapsibleContent,
} from "@/components/ui/collapsible";
import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import {
  ChevronDownIcon,
  DownloadIcon,
  LightbulbIcon,
  TerminalIcon,
  WrenchIcon,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import type {
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

type RuntimeSectionHeaderProps = {
  kind: "reasoning" | "tools" | "terminal";
};

const RuntimeSectionHeader = ({ kind }: RuntimeSectionHeaderProps) => {
  const { duration, isOpen, isStreaming } = useReasoning();
  const [elapsedSeconds, setElapsedSeconds] = useState(0);

  useEffect(() => {
    if (!isStreaming) {
      return;
    }

    const timer = window.setInterval(() => {
      setElapsedSeconds((prev) => prev + 1);
    }, 1000);

    return () => window.clearInterval(timer);
  }, [isStreaming]);

  const finalSeconds = Math.max(0, duration ?? elapsedSeconds);
  const hasDuration = duration !== undefined || elapsedSeconds > 0;
  const headerConfig =
    kind === "reasoning"
      ? {
          Icon: LightbulbIcon,
          idleLabel: "Reasoning",
          streamingLabel: "Thinking...",
          completedLabel: `Thought for ${finalSeconds}s`,
        }
      : kind === "tools"
        ? {
            Icon: WrenchIcon,
            idleLabel: "Tools",
            streamingLabel: "Running tools...",
            completedLabel: `Tools ran for ${finalSeconds}s`,
          }
        : {
            Icon: TerminalIcon,
            idleLabel: "Terminal",
            streamingLabel: "Running terminal...",
            completedLabel: `Terminal ran for ${finalSeconds}s`,
          };

  const titleLabel = isStreaming
    ? headerConfig.streamingLabel
    : hasDuration
      ? headerConfig.completedLabel
      : headerConfig.idleLabel;

  return (
    <>
      <headerConfig.Icon className="size-3.5 text-muted-foreground" />
      <span className="text-xs font-medium leading-4 text-muted-foreground">
        {isStreaming ? (
          <Shimmer as="span" duration={1}>
            {titleLabel}
          </Shimmer>
        ) : (
          titleLabel
        )}
      </span>
      {isStreaming && (
        <span className="shrink-0 text-[11px] leading-4 text-muted-foreground">
          {elapsedSeconds}s
        </span>
      )}
      <ChevronDownIcon
        className={cn(
          "size-3.5 text-muted-foreground transition-transform",
          isOpen && "rotate-180"
        )}
      />
    </>
  );
};

const TerminalDownloadButton = ({ output }: { output: string }) => {
  const isDisabled = output.trim().length === 0;

  const downloadOutput = useCallback(() => {
    const blob = new Blob([output], { type: "text/plain;charset=utf-8" });
    const url = window.URL.createObjectURL(blob);
    const link = window.document.createElement("a");
    link.href = url;
    link.download = "terminal-output.txt";
    window.document.body.append(link);
    link.click();
    link.remove();
    window.URL.revokeObjectURL(url);
  }, [output]);

  return (
    <Button
      aria-label="Download terminal output"
      className="size-5 shrink-0 border-0 bg-transparent text-muted-foreground shadow-none hover:bg-muted/60 hover:text-foreground [&_svg]:size-[11px]"
      disabled={isDisabled}
      onClick={downloadOutput}
      size="icon-sm"
      title="Download terminal output"
      variant="ghost"
    >
      <DownloadIcon size={11} />
    </Button>
  );
};

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
    const reasoningExpanded = turnUiState?.sectionExpanded.reasoning;
    const isReasoningControlled = typeof reasoningExpanded === "boolean";

    return (
      <Reasoning
        className="mb-0"
        defaultOpen={section.defaultOpen}
        isStreaming={section.isStreaming}
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isReasoningControlled ? reasoningExpanded : undefined}
      >
        <ReasoningTrigger className="h-8 justify-start px-2.5 py-1.5 hover:text-foreground">
          <RuntimeSectionHeader
            key={`reasoning-${section.isStreaming ? "stream" : "idle"}`}
            kind="reasoning"
          />
        </ReasoningTrigger>
        <ReasoningContent className="llm-chat-markdown mt-0 px-2.5 py-2 text-[12px] leading-5 text-foreground/90">
          {turn.reasoning.text || "Streaming reasoning..."}
        </ReasoningContent>
      </Reasoning>
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
    const toolsExpanded = turnUiState?.sectionExpanded.tools;
    const isToolsControlled = typeof toolsExpanded === "boolean";

    return (
      <Reasoning
        className="mb-0"
        defaultOpen={false}
        isStreaming={section.isStreaming}
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isToolsControlled ? toolsExpanded : undefined}
      >
        <ReasoningTrigger className="h-8 justify-start px-2.5 py-1.5 hover:text-foreground">
          <RuntimeSectionHeader
            key={`tools-${section.isStreaming ? "stream" : "idle"}`}
            kind="tools"
          />
        </ReasoningTrigger>
        <CollapsibleContent className="llm-chat-process-tools data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 space-y-1.5 px-2.5 py-2 text-muted-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in">
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
                defaultOpen={false}
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
      </Reasoning>
    );
  };

  const renderTerminal = (section: TurnProcessSectionVM) => {
    const terminalExpanded = turnUiState?.sectionExpanded.terminal;
    const isTerminalControlled = typeof terminalExpanded === "boolean";
    const terminalOutputKey = `${turn.id}:terminal`;
    const outputExpanded = Boolean(turnUiState?.codeExpanded[terminalOutputKey]);
    const terminalOutput = vm.terminalOutput;
    const hasTerminalOutput = terminalOutput.trim().length > 0;

    return (
      <Reasoning
        className="mb-0"
        defaultOpen={false}
        isStreaming={section.isStreaming}
        key={section.key}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isTerminalControlled ? terminalExpanded : undefined}
      >
        <ReasoningTrigger className="h-8 justify-start px-2.5 py-1.5 hover:text-foreground">
          <RuntimeSectionHeader
            key={`terminal-${section.isStreaming ? "stream" : "idle"}`}
            kind="terminal"
          />
        </ReasoningTrigger>
        <CollapsibleContent className="px-2.5 py-2">
          <Terminal
            className="border-0 rounded-none bg-transparent"
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
            output={terminalOutput}
            previewLines={8}
          >
            <div className="llm-chat-terminal-surface overflow-hidden rounded-md bg-muted/50">
              <div className="flex items-center justify-between px-2 py-0.5">
                <span className="inline-flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground">
                  <TerminalIcon className="size-3.5" />
                  Terminal
                </span>
                <span className="inline-flex items-center gap-1">
                  <TerminalCopyButton
                    aria-label="Copy terminal output"
                    className="size-5 border-0 bg-transparent text-muted-foreground shadow-none hover:bg-muted/60 hover:text-foreground [&_svg]:size-[11px]"
                    disabled={!hasTerminalOutput}
                    title="Copy terminal output"
                  />
                  <TerminalDownloadButton output={terminalOutput} />
                </span>
              </div>
              <TerminalContent className="!max-h-56 !bg-transparent !px-2 !pb-1.5 !pt-0.5 !text-[12px] !leading-[1.4]" />
            </div>
          </Terminal>
        </CollapsibleContent>
      </Reasoning>
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
