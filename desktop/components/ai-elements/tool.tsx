"use client";

import type { DynamicToolUIPart, ToolUIPart } from "@/types";
import type { ComponentProps, ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import {
  CheckCircleIcon,
  ChevronDownIcon,
  CircleIcon,
  ClockIcon,
  WrenchIcon,
  XCircleIcon,
} from "lucide-react";
import { isValidElement } from "react";

import {
  CodeBlock,
  CodeBlockActions,
  CodeBlockCopyButton,
  CodeBlockFilename,
  CodeBlockHeader,
  CodeBlockTitle,
} from "./code-block";
import {
  COLLAPSIBLE_CONTENT_ANIMATION_CLASS,
  SURFACE_ICON_GHOST_BUTTON_CLASS,
} from "./class-names";

export type ToolProps = ComponentProps<typeof Collapsible> & {
  compact?: boolean;
};

export const Tool = ({ className, compact = false, ...props }: ToolProps) => (
  <Collapsible
    className={cn(
      "group not-prose w-full rounded-md border",
      compact ? "mb-1.5 border-0 bg-transparent" : "mb-4",
      className
    )}
    {...props}
  />
);

export type ToolPart = ToolUIPart | DynamicToolUIPart;

export type ToolHeaderProps = {
  title?: string;
  className?: string;
  compact?: boolean;
} & (
  | { type: ToolUIPart["type"]; state: ToolUIPart["state"]; toolName?: never }
  | {
      type: DynamicToolUIPart["type"];
      state: DynamicToolUIPart["state"];
      toolName: string;
    }
);

const statusLabels: Record<ToolPart["state"], string> = {
  "approval-requested": "Awaiting Approval",
  "approval-responded": "Responded",
  "input-available": "Running",
  "input-streaming": "Pending",
  "output-available": "Completed",
  "output-denied": "Denied",
  "output-error": "Error",
};

const getStatusIcon = (status: ToolPart["state"], compact = false): ReactNode => {
  const sizeClass = compact ? "size-3.5" : "size-4";
  if (status === "approval-requested") {
    return <ClockIcon className={cn(sizeClass, "text-yellow-600")} />;
  }
  if (status === "approval-responded") {
    return <CheckCircleIcon className={cn(sizeClass, "text-blue-600")} />;
  }
  if (status === "input-available") {
    return <ClockIcon className={cn(sizeClass, "animate-pulse")} />;
  }
  if (status === "input-streaming") {
    return <CircleIcon className={sizeClass} />;
  }
  if (status === "output-available") {
    return <CheckCircleIcon className={cn(sizeClass, "text-green-600")} />;
  }
  if (status === "output-denied") {
    return <XCircleIcon className={cn(sizeClass, "text-orange-600")} />;
  }
  return <XCircleIcon className={cn(sizeClass, "text-red-600")} />;
};

export const getStatusBadge = (
  status: ToolPart["state"],
  compact = false
) => (
  <Badge
    className={cn(
      "rounded-full",
      compact ? "gap-1 px-1.5 py-0 text-[10px]" : "gap-1.5 text-xs"
    )}
    variant="secondary"
  >
    {getStatusIcon(status, compact)}
    {statusLabels[status]}
  </Badge>
);

export const ToolHeader = ({
  className,
  compact = false,
  title,
  type,
  state,
  toolName,
  ...props
}: ToolHeaderProps) => {
  const derivedName =
    type === "dynamic-tool" ? toolName : type.split("-").slice(1).join("-");

  return (
    <CollapsibleTrigger
      className={cn(
        "flex w-full items-center justify-between",
        compact ? "gap-2 px-2.5 py-1.5" : "gap-4 p-3",
        className
      )}
      {...props}
    >
      <div className="min-w-0 flex items-center gap-2">
        <WrenchIcon className={cn(compact ? "size-3.5" : "size-4", "text-muted-foreground")} />
        <span className={cn("truncate font-medium", compact ? "text-xs" : "text-sm")}>
          {title ?? derivedName}
        </span>
        {getStatusBadge(state, compact)}
      </div>
      <ChevronDownIcon
        className={cn(
          compact ? "size-3.5" : "size-4",
          "text-muted-foreground transition-transform group-data-[state=open]:rotate-180"
        )}
      />
    </CollapsibleTrigger>
  );
};

export type ToolContentProps = ComponentProps<typeof CollapsibleContent> & {
  compact?: boolean;
};

export const ToolContent = ({
  className,
  compact = false,
  ...props
}: ToolContentProps) => (
  <CollapsibleContent
    className={cn(
      COLLAPSIBLE_CONTENT_ANIMATION_CLASS,
      "text-popover-foreground",
      compact ? "space-y-2 px-2.5 py-2" : "space-y-4 p-4",
      className
    )}
    {...props}
  />
);

export type ToolInputProps = ComponentProps<"div"> & {
  input: ToolPart["input"];
  compact?: boolean;
};

const ToolCodeBlock = ({
  code,
  compact = false,
  language = "json",
}: {
  code: string;
  compact?: boolean;
  language?: "json";
}) => (
  <CodeBlock
    className="llm-chat-code-surface llm-chat-runtime-surface"
    code={code}
    compact={compact}
    language={language}
  >
    <CodeBlockHeader
      className={cn(
        "border-0 bg-transparent",
        compact
          ? "px-[var(--chat-runtime-code-padding-x)] pt-[var(--chat-runtime-code-padding-y)] pb-1"
          : "px-[var(--chat-runtime-code-padding-x)] pt-[var(--chat-runtime-code-padding-y)] pb-1"
      )}
    >
      <CodeBlockTitle className="gap-1.5 text-[var(--chat-runtime-surface-label)]">
        <CodeBlockFilename className={cn(compact ? "text-[11px]" : "text-xs")}>
          {language}
        </CodeBlockFilename>
      </CodeBlockTitle>
      <CodeBlockActions className="-my-0.5 gap-1">
        <CodeBlockCopyButton className={SURFACE_ICON_GHOST_BUTTON_CLASS} />
      </CodeBlockActions>
    </CodeBlockHeader>
  </CodeBlock>
);

export const ToolInput = ({
  className,
  input,
  compact = false,
  ...props
}: ToolInputProps) => (
  <div className={cn(compact ? "space-y-1.5 overflow-hidden" : "space-y-2 overflow-hidden", className)} {...props}>
    <h4 className={cn("font-medium text-muted-foreground uppercase tracking-wide", compact ? "text-[11px]" : "text-xs")}>
      Parameters
    </h4>
    <ToolCodeBlock code={JSON.stringify(input, null, 2)} compact={compact} />
  </div>
);

const lineCount = (value: string): number => {
  if (!value) {
    return 0;
  }
  return value.split("\n").length;
};

const truncateLines = (value: string, maxLines: number): string =>
  value.split("\n").slice(0, maxLines).join("\n");

export type ToolOutputProps = ComponentProps<"div"> & {
  output: ToolPart["output"];
  errorText: ToolPart["errorText"];
  compact?: boolean;
  previewLines?: number;
  expanded?: boolean;
  onToggleExpand?: () => void;
};

export const ToolOutput = ({
  className,
  output,
  errorText,
  compact = false,
  previewLines,
  expanded = false,
  onToggleExpand,
  ...props
}: ToolOutputProps) => {
  if (!(output || errorText)) {
    return null;
  }

  const outputAsCode =
    typeof output === "object" && output !== null && !isValidElement(output)
      ? JSON.stringify(output, null, 2)
      : typeof output === "string"
        ? output
        : undefined;
  const hasOutput = output !== undefined && output !== null && output !== "";

  const totalLines = outputAsCode ? lineCount(outputAsCode) : 0;
  const hasPreview =
    typeof previewLines === "number" &&
    previewLines > 0 &&
    totalLines > previewLines;
  const showPreview = Boolean(hasPreview && !expanded && outputAsCode);
  const displayCode = outputAsCode
    ? showPreview
      ? truncateLines(outputAsCode, previewLines ?? 0)
      : outputAsCode
    : "";

  let outputNode: ReactNode = <div>{output as ReactNode}</div>;
  if (outputAsCode) {
    outputNode = <ToolCodeBlock code={displayCode} compact={compact} />;
  }

  return (
    <div className={cn(compact ? "space-y-1.5" : "space-y-2", className)} {...props}>
      <h4
        className={cn(
          "font-medium text-muted-foreground uppercase tracking-wide",
          compact ? "text-[11px]" : "text-xs"
        )}
      >
        {errorText ? "Error" : "Result"}
      </h4>
      {errorText && (
        <div
          className={cn(
            "rounded-md bg-destructive/10 text-destructive",
            compact ? "px-2 py-1.5 text-[11px]" : "px-3 py-2 text-xs"
          )}
        >
          {errorText}
        </div>
      )}
      {outputAsCode ? (
        outputNode
      ) : (
        hasOutput && (
          <div
            className={cn(
              "llm-chat-runtime-surface overflow-x-auto px-[var(--chat-runtime-code-padding-x)] py-[var(--chat-runtime-code-padding-y)] text-[var(--chat-runtime-surface-text)] [&_table]:w-full",
              compact ? "text-[11px]" : "text-xs"
            )}
          >
            {outputNode}
          </div>
        )
      )}
      {hasPreview && onToggleExpand && (
        <Button
          className={cn("h-6 px-2 text-[11px]")}
          onClick={onToggleExpand}
          size="sm"
          type="button"
          variant="ghost"
        >
          {expanded ? "Show less" : `Show all (${totalLines} lines)`}
        </Button>
      )}
    </div>
  );
};
