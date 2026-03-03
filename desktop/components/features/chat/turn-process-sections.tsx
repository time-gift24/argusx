"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import {
  CodeBlockActions,
  CodeBlockFilename,
  CodeBlockHeader,
  CodeBlockTitle,
} from "@/components/ai-elements/code-block";
import {
  Plan,
  PlanAction,
  PlanContent,
  PlanDescription,
  PlanHeader,
  PlanTitle,
  PlanTrigger,
} from "@/components/ai-elements/plan";
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
import { SURFACE_ICON_GHOST_BUTTON_CLASS } from "@/components/ai-elements/class-names";
import { STREAMDOWN_PLUGINS } from "@/components/ai-elements/streamdown-plugins";
import { StreamdownCode } from "@/components/ai-elements/streamdown-code";
import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";
import { Streamdown } from "streamdown";
import {
  LightbulbIcon,
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
      <Streamdown BlockComponent={RuntimeMarkdownBlock} plugins={STREAMDOWN_PLUGINS}>
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
