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

import { CodeBlock } from "./code-block";

export type ToolProps = ComponentProps<typeof Collapsible> & {
  compact?: boolean;
};

export const Tool = ({ className, compact = false, ...props }: ToolProps) => (
  <Collapsible
    className={cn(
      "group not-prose w-full rounded-md border",
      compact ? "mb-1.5 border-border/60 bg-background/50" : "mb-4",
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
      "data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 text-popover-foreground outline-none data-[state=closed]:animate-out data-[state=open]:animate-in",
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
    <div className="rounded-md bg-muted/50">
      <CodeBlock compact={compact} code={JSON.stringify(input, null, 2)} language="json" />
    </div>
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
    outputNode = <CodeBlock compact={compact} code={displayCode} language="json" />;
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
      <div
        className={cn(
          "overflow-x-auto rounded-md [&_table]:w-full",
          compact ? "text-[11px]" : "text-xs",
          errorText
            ? "bg-destructive/10 text-destructive"
            : "bg-muted/50 text-foreground"
        )}
      >
        {errorText && (
          <div className={cn(compact ? "px-2 py-1.5" : "px-3 py-2")}>{errorText}</div>
        )}
        {outputNode}
      </div>
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
