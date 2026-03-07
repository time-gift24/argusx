"use client";

import { AlertCircleIcon, WrenchIcon } from "lucide-react";

import {
  RuntimeCodePanel,
  type RuntimeCodeSection,
} from "@/components/ai/runtime-code-panel";
import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import { AI_RUNTIME_DENSITY } from "@/components/ai/styles";
import { cn } from "@/lib/utils";

export type ToolCallItemProps = Omit<StreamItemProps, "children"> & {
  sections?: RuntimeCodeSection[];
  name: string;
  inputSummary?: string;
  outputSummary?: string;
  errorSummary?: string;
};

function SummaryBlock({
  label,
  children,
}: {
  label: string;
  children: string;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span
        className={cn(
          AI_RUNTIME_DENSITY.eyebrowText,
          "text-muted-foreground/80"
        )}
      >
        {label}
      </span>
      <p className={cn(AI_RUNTIME_DENSITY.bodyText, "text-foreground/85")}>
        {children}
      </p>
    </div>
  );
}

export function ToolCallItem({
  className,
  errorSummary,
  inputSummary,
  isRunning = false,
  name,
  outputSummary,
  sections,
  ...props
}: ToolCallItemProps) {
  const status = errorSummary
    ? "Failed"
    : isRunning
      ? "Running"
      : "Completed";

  const Icon = errorSummary ? AlertCircleIcon : WrenchIcon;

  if (sections?.length) {
    return (
      <RuntimeCodePanel
        {...props}
        className={className}
        icon={
          <Icon
            className={cn(
              "size-4",
              errorSummary ? "text-destructive" : undefined
            )}
          />
        }
        isRunning={isRunning}
        sections={sections}
        status={status}
        title={name}
      />
    );
  }

  return (
    <StreamItem className={className} isRunning={isRunning} {...props}>
      <StreamItemTrigger
        icon={
          <Icon
            className={cn(
              "size-4",
              errorSummary ? "text-destructive" : undefined
            )}
          />
        }
        label={name}
        status={status}
      />
      <StreamItemContent
        className={cn(
          "flex flex-col",
          AI_RUNTIME_DENSITY.sectionGap,
          AI_RUNTIME_DENSITY.bodyText
        )}
      >
        {inputSummary ? <SummaryBlock label="Input">{inputSummary}</SummaryBlock> : null}
        {outputSummary ? (
          <SummaryBlock label="Output">{outputSummary}</SummaryBlock>
        ) : null}
        {errorSummary ? (
          <SummaryBlock label="Error">{errorSummary}</SummaryBlock>
        ) : null}
      </StreamItemContent>
    </StreamItem>
  );
}
