"use client";

import { AlertCircleIcon, WrenchIcon } from "lucide-react";

import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import { cn } from "@/lib/utils";

export type ToolCallItemProps = Omit<StreamItemProps, "children"> & {
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
      <span className="text-[11px] uppercase tracking-[0.08em] text-muted-foreground/80">
        {label}
      </span>
      <p className="text-foreground/85">{children}</p>
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
  ...props
}: ToolCallItemProps) {
  const status = errorSummary
    ? "Failed"
    : isRunning
      ? "Running"
      : "Completed";

  const Icon = errorSummary ? AlertCircleIcon : WrenchIcon;

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
      <StreamItemContent className="flex flex-col gap-3 leading-6">
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
