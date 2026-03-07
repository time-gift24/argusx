"use client";

import { BrainIcon } from "lucide-react";
import { Streamdown } from "streamdown";

import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import {
  sharedStreamdownClassName,
  sharedStreamdownComponents,
  sharedStreamdownControls,
  sharedStreamdownIcons,
  sharedStreamdownPlugins,
  sharedStreamdownTranslations,
} from "@/components/ai/streamdown";
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
        icon={<BrainIcon className="size-[10px]" />}
        label="Reasoning"
        status={isRunning ? "Thinking" : "Thought"}
      />
      <StreamItemContent
        className={cn(
          AI_RUNTIME_DENSITY.bodyText,
          contentClassName
        )}
      >
        <Streamdown
          className={sharedStreamdownClassName}
          components={sharedStreamdownComponents}
          controls={sharedStreamdownControls}
          icons={sharedStreamdownIcons}
          isAnimating={isRunning}
          plugins={sharedStreamdownPlugins}
          translations={sharedStreamdownTranslations}
        >
          {children}
        </Streamdown>
      </StreamItemContent>
    </StreamItem>
  );
}
