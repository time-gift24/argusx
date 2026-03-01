"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import {
  Plan,
  PlanAction,
  PlanContent,
  PlanDescription,
  PlanHeader,
  PlanTitle,
  PlanTrigger,
} from "@/components/ai-elements/plan";
import { Queue, QueueItem, QueueItemContent, QueueItemIndicator, QueueList } from "@/components/ai-elements/queue";
import { Task, TaskContent, TaskItem, TaskTrigger } from "@/components/ai-elements/task";
import { Terminal } from "@/components/ai-elements/terminal";
import { Message, MessageResponse } from "@/components/ai-elements/message";
import { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from "@/components/ai-elements/tool";
import { Button } from "@/components/ui/button";
import { useChatStore } from "@/lib/stores/chat-store";
import { ChevronDownIcon, LightbulbIcon, Loader2Icon } from "lucide-react";
import { useMemo } from "react";

interface AgentTurnCardProps {
  sessionId: string;
  turn: AgentTurnVM;
}

const reasoningStatusLabel: Record<AgentTurnVM["reasoning"]["status"], string> = {
  idle: "Idle",
  started: "Started",
  streaming: "Streaming",
  completed: "Completed",
  error: "Error",
};

const queueStatusLabel: Record<AgentTurnVM["queue"]["items"][number]["status"], string> = {
  waiting: "Waiting",
  running: "Running",
  completed: "Completed",
  failed: "Failed",
};

export function AgentTurnCard({ sessionId, turn }: AgentTurnCardProps) {
  const setReasoningExpanded = useChatStore((state) => state.setReasoningExpanded);

  const terminalOutput = useMemo(() => {
    const sections: string[] = [];
    if (turn.terminal.stdout.trim().length > 0) {
      sections.push(`stdout:\n${turn.terminal.stdout}`);
    }
    if (turn.terminal.stderr.trim().length > 0) {
      sections.push(`stderr:\n${turn.terminal.stderr}`);
    }
    if (sections.length === 0 && turn.terminal.output.trim().length > 0) {
      sections.push(turn.terminal.output);
    }
    if (turn.terminal.exitCode !== undefined) {
      sections.push(
        `exit_code: ${turn.terminal.exitCode} (${turn.terminal.durationMs ?? 0}ms)`
      );
    }
    return sections.join("\n\n");
  }, [turn.terminal]);

  const hasReasoning = turn.reasoning.text.trim().length > 0 || turn.reasoning.isStreaming;
  const hasTerminal = terminalOutput.trim().length > 0 || turn.terminal.isStreaming;

  return (
    <Message from="assistant">
      <div className="w-full space-y-3">
        {hasReasoning && (
          <div className="overflow-hidden rounded-xl border border-border/80 bg-background/70 shadow-xs transition-opacity duration-200 motion-reduce:transition-none">
            <Button
              className="flex h-auto w-full items-start justify-between rounded-none px-4 py-3 text-left"
              onClick={() =>
                setReasoningExpanded(sessionId, turn.id, !turn.reasoning.isExpanded)
              }
              type="button"
              variant="ghost"
            >
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-sm">
                  <LightbulbIcon className="size-4 text-muted-foreground" />
                  <span className="font-medium">Reasoning</span>
                  {turn.reasoning.isStreaming && (
                    <Loader2Icon className="size-3.5 animate-spin text-muted-foreground" />
                  )}
                  <span className="text-muted-foreground text-xs">
                    {reasoningStatusLabel[turn.reasoning.status]} · {turn.reasoning.charCount} chars
                    {turn.reasoning.truncated ? " · truncated" : ""}
                  </span>
                </div>
                {!turn.reasoning.isExpanded && (
                  <p className="line-clamp-2 text-muted-foreground text-xs">
                    {turn.reasoning.preview || "Streaming reasoning..."}
                  </p>
                )}
              </div>
              <ChevronDownIcon
                className={`size-4 text-muted-foreground transition-transform ${
                  turn.reasoning.isExpanded ? "rotate-180" : ""
                }`}
              />
            </Button>
            {turn.reasoning.isExpanded && (
              <div className="max-h-72 overflow-auto border-border/60 border-t px-4 py-3 text-sm leading-relaxed">
                <p className="whitespace-pre-wrap">{turn.reasoning.text}</p>
              </div>
            )}
          </div>
        )}

        {turn.plan && (
          <Plan defaultOpen={!turn.plan.isStreaming} isStreaming={turn.plan.isStreaming}>
            <PlanHeader>
              <div className="space-y-1">
                <PlanTitle>{turn.plan.title}</PlanTitle>
                {turn.plan.description && (
                  <PlanDescription>{turn.plan.description}</PlanDescription>
                )}
              </div>
              <PlanAction>
                <PlanTrigger />
              </PlanAction>
            </PlanHeader>
            <PlanContent className="space-y-2">
              {turn.plan.tasks.map((task) => (
                <TaskItem key={task.id}>
                  <span className="inline-flex items-center gap-2">
                    <span>{task.status === "completed" ? "✓" : "•"}</span>
                    <span>{task.title}</span>
                  </span>
                </TaskItem>
              ))}
            </PlanContent>
          </Plan>
        )}

        {turn.plan && (
          <Task defaultOpen={false}>
            <TaskTrigger title={`Tasks (${turn.plan.tasks.length})`} />
            <TaskContent>
              {turn.plan.tasks.map((task) => (
                <TaskItem className={task.status === "completed" ? "line-through" : ""} key={task.id}>
                  {task.title}
                </TaskItem>
              ))}
            </TaskContent>
          </Task>
        )}

        {turn.tools.map((tool) => (
          <Tool defaultOpen={tool.state === "output-error"} key={tool.callId}>
            <ToolHeader state={tool.state} toolName={tool.toolName} type="dynamic-tool" />
            <ToolContent>
              {tool.input && <ToolInput input={tool.input} />}
              <ToolOutput errorText={tool.errorText} output={tool.output} />
            </ToolContent>
          </Tool>
        ))}

        {turn.queue.items.length > 0 && (
          <Queue>
            <div className="px-1 py-1 font-medium text-muted-foreground text-xs uppercase tracking-wide">
              Tool Queue
            </div>
            <QueueList>
              {turn.queue.items.map((item) => (
                <QueueItem key={item.callId}>
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

        {hasTerminal && <Terminal isStreaming={turn.terminal.isStreaming} output={terminalOutput} />}

        {turn.assistantText.trim().length > 0 && (
          <MessageResponse>{turn.assistantText}</MessageResponse>
        )}

        {turn.status === "failed" || turn.status === "cancelled" ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-sm">
            {turn.error ?? "Turn failed"}
          </div>
        ) : null}
      </div>
    </Message>
  );
}
