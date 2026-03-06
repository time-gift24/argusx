"use client";

import { BrainIcon } from "lucide-react";
import { cjk } from "@streamdown/cjk";
import { code } from "@streamdown/code";
import { math } from "@streamdown/math";
import { mermaid } from "@streamdown/mermaid";
import { Streamdown } from "streamdown";

import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import { cn } from "@/lib/utils";

const streamdownPlugins = { cjk, code, math, mermaid };

export type ReasoningProps = Omit<StreamItemProps, "children"> & {
  children: string;
  contentClassName?: string;
};

export function Reasoning({
  children,
  className,
  contentClassName,
  isRunning = false,
  ...props
}: ReasoningProps) {
  return (
    <StreamItem className={className} isRunning={isRunning} {...props}>
      <StreamItemTrigger
        icon={<BrainIcon className="size-4" />}
        label="Reasoning"
        status={isRunning ? "Thinking" : "Thought"}
      />
      <StreamItemContent className={cn("leading-6", contentClassName)}>
        <Streamdown plugins={streamdownPlugins}>{children}</Streamdown>
      </StreamItemContent>
    </StreamItem>
  );
}
