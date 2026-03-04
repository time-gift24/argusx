"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import {
  CodeBlockActions,
  CodeBlockFilename,
  CodeBlockHeader,
  CodeBlockTitle,
} from "@/components/ai/code-block";
import {
  Plan,
  PlanAction,
  PlanContent,
  PlanDescription,
  PlanHeader,
  PlanTitle,
  PlanTrigger,
} from "@/components/ai/plan";
import { TaskItem } from "@/components/ai-elements/task";
import {
  Terminal,
  TerminalContent,
  TerminalCopyButton,
} from "@/components/ai/terminal";
import {
  Tool,
  ToolContent,
  ToolHeader,
  ToolInput,
  ToolOutput,
} from "@/components/ai/tool";
import { Queue } from "@/components/ai-elements/queue";
import { SURFACE_ICON_GHOST_BUTTON_CLASS } from "@/components/ai-elements/class-names";
import { STREAMDOWN_PLUGINS, StreamdownCode } from "@/components/ai";
import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import { Streamdown } from "streamdown";
import {
  LightbulbIcon,
  ListTodoIcon,
  TerminalIcon,
  WrenchIcon,
} from "lucide-react";

import { RuntimeProcessSection } from "./runtime-process-section";
import type {
  TurnProcessCompactItemVM,
  TurnProcessSectionVM,
  TurnProcessVM,
} from "./turn-process-view-model";

interface TurnProcessSectionsProps {
  sessionId: string;
  turn: AgentTurnVM;
  vm: TurnProcessVM;
}

const statusDotClass: Record<TurnProcessCompactItemVM["status"], string> = {
  Waiting: "bg-zinc-400",
  Running: "bg-blue-500",
  Completed: "bg-emerald-500",
  Failed: "bg-destructive",
};

const renderCompactItems = (items: TurnProcessCompactItemVM[] | undefined) => {
  if (!items || items.length === 0) {
    return null;
  }

  return (
    <div className="llm-chat-runtime-compact-list space-y-1">
      {items.map((item) => (
        <div className="flex min-w-0 items-center gap-1.5 text-[11px] leading-4" key={item.id}>
          <span
            aria-hidden
            className={cn("size-1.5 shrink-0 rounded-full", statusDotClass[item.status])}
          />
          <span className="truncate">{item.label}</span>
          <span className="shrink-0 text-[10px] uppercase tracking-wide text-[var(--chat-runtime-text-secondary)]/90">
            {item.status}
          </span>
        </div>
      ))}
    </div>
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

  const renderReasoning = (section: TurnProcessSectionVM) => (
    <RuntimeProcessSection
      icon={LightbulbIcon}
      isStreaming={section.isStreaming}
      key={section.key}
      label={section.headerLabel}
      onOpenChange={(nextOpen) =>
        setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
      }
      open={isSectionOpen(section)}
      detail={section.headerDetail}
      contentClassName="llm-chat-markdown px-[var(--chat-runtime-code-padding-x)] py-[var(--chat-runtime-code-padding-y)] text-[13px] leading-5 text-foreground/90"
    >
      <Streamdown
        components={{ code: StreamdownCode }}
        plugins={STREAMDOWN_PLUGINS}
      >
        {turn.reasoning.text || "Streaming reasoning..."}
      </Streamdown>
    </RuntimeProcessSection>
  );

  const renderPlan = (section: TurnProcessSectionVM) => {
    if (!turn.plan) {
      return null;
    }

    return (
      <Plan
        className="plan-surface"
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

  const renderTools = (section: TurnProcessSectionVM) => (
    <RuntimeProcessSection
      icon={WrenchIcon}
      isStreaming={section.isStreaming}
      key={section.key}
      label={section.headerLabel}
      onOpenChange={(nextOpen) =>
        setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
      }
      open={isSectionOpen(section)}
      detail={section.headerDetail}
      contentClassName="llm-chat-process-tools space-y-1.5 px-[var(--chat-runtime-code-padding-x)] py-[var(--chat-runtime-code-padding-y)] text-[var(--chat-runtime-text-secondary)]"
    >
      {renderCompactItems(section.compactItems)}
      {turn.tools.map((tool) => {
        const toolOutputKey = `${turn.id}:tool:${tool.callId}`;
        const outputExpanded = Boolean(turnUiState?.codeExpanded[toolOutputKey]);

        return (
          <Tool compact defaultOpen={false} key={tool.callId}>
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
      {turn.subAgents.length > 0 && (
        <div className="space-y-2 rounded-md border border-border/60 px-3 py-2">
          {turn.subAgents.map((subAgent) => {
            const activeTool = [...subAgent.tools]
              .sort((a, b) => b.updatedAt - a.updatedAt)
              .at(0);
            const detail = activeTool
              ? `${activeTool.toolName} · ${activeTool.status}`
              : subAgent.status;
            return (
              <div
                className="flex items-center justify-between gap-3 text-xs text-[var(--chat-runtime-text-secondary)]"
                key={subAgent.threadId}
              >
                <div className="min-w-0 truncate">
                  <span className="font-medium text-foreground/90">
                    {subAgent.agentName}
                  </span>
                  <span className="ml-2 text-[11px] opacity-80">{subAgent.threadId}</span>
                </div>
                <div className="shrink-0 text-[11px] uppercase tracking-wide opacity-90">
                  {detail}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </RuntimeProcessSection>
  );

  const renderTerminal = (section: TurnProcessSectionVM) => {
    const terminalOutputKey = `${turn.id}:terminal`;
    const outputExpanded = Boolean(turnUiState?.codeExpanded[terminalOutputKey]);
    const terminalOutput = vm.terminalOutput;
    const hasTerminalOutput = terminalOutput.trim().length > 0;

    return (
      <RuntimeProcessSection
        icon={TerminalIcon}
        isStreaming={section.isStreaming}
        key={section.key}
        label={section.headerLabel}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isSectionOpen(section)}
        detail={section.headerDetail}
        contentClassName="px-[var(--chat-runtime-code-padding-x)] py-[var(--chat-runtime-code-padding-y)]"
      >
        <Terminal
          className="bg-transparent"
          compact
          expanded={outputExpanded}
          isStreaming={turn.terminal.isStreaming}
          onToggleExpand={(expanded) =>
            setTurnCodeExpanded(sessionId, turn.id, terminalOutputKey, expanded)
          }
          output={terminalOutput}
          previewLines={8}
        >
          <div className="llm-chat-code-surface llm-chat-terminal-surface llm-chat-runtime-surface overflow-hidden">
            <CodeBlockHeader className="border-0 bg-transparent px-[var(--chat-runtime-code-padding-x)] pt-[var(--chat-runtime-code-padding-y)] pb-0">
              <CodeBlockTitle className="text-[var(--chat-runtime-surface-label)]">
                <CodeBlockFilename className="[font-size:var(--chat-runtime-code-font-size)]">
                  terminal
                </CodeBlockFilename>
              </CodeBlockTitle>
              <CodeBlockActions className="-my-0.5 gap-1">
                <TerminalCopyButton
                  aria-label="Copy terminal output"
                  className={cn(
                    SURFACE_ICON_GHOST_BUTTON_CLASS,
                    "[&_svg]:size-[14px]"
                  )}
                  disabled={!hasTerminalOutput}
                  title="Copy terminal output"
                />
              </CodeBlockActions>
            </CodeBlockHeader>
            <TerminalContent className="!max-h-56 !bg-transparent !px-[var(--chat-runtime-code-padding-x)] !pb-[var(--chat-runtime-code-padding-y)] !pt-0 ![font-size:var(--chat-runtime-code-font-size)] ![line-height:var(--chat-runtime-code-line-height)]" />
          </div>
        </Terminal>
      </RuntimeProcessSection>
    );
  };

  const renderQueue = (section: TurnProcessSectionVM) => {
    if (!turn.todoQueue) {
      return null;
    }

    return (
      <RuntimeProcessSection
        icon={ListTodoIcon}
        isStreaming={section.isStreaming}
        key={section.key}
        label={section.headerLabel || section.title}
        onOpenChange={(nextOpen) =>
          setTurnSectionExpanded(sessionId, turn.id, section.key, nextOpen)
        }
        open={isSectionOpen(section)}
        detail={section.headerDetail}
        contentClassName="px-[var(--chat-runtime-code-padding-x)] py-[var(--chat-runtime-code-padding-y)]"
      >
        <Queue todos={turn.todoQueue.todos} compact />
      </RuntimeProcessSection>
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
        if (section.key === "terminal") {
          return renderTerminal(section);
        }
        if (section.key === "queue") {
          return renderQueue(section);
        }
        return null;
      })}
    </div>
  );
}
