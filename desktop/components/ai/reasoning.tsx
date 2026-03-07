"use client";

import { BrainIcon } from "lucide-react";
import { Streamdown } from "streamdown";

import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import { AI_RUNTIME_DENSITY } from "@/components/ai/styles";
import { cn } from "@/lib/utils";

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
        icon={<BrainIcon />}
        label="reasoning"
        status={isRunning ? "Thinking" : "Thought"}
      />
      <StreamItemContent
        className={cn(
          AI_RUNTIME_DENSITY.bodyText,
          contentClassName
        )}
      >
        <Streamdown isAnimating={isRunning}>
          {children}
        </Streamdown>
      </StreamItemContent>
    </StreamItem>
  );
}
