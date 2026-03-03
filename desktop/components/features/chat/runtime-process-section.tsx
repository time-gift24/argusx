"use client";

import type { ComponentProps, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

import { Shimmer } from "@/components/ai-elements/shimmer";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { ChevronDownIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { COLLAPSIBLE_CONTENT_ANIMATION_CLASS } from "@/components/ai-elements/class-names";

const MS_IN_SECOND = 1000;

type RuntimeProcessSectionProps = Omit<ComponentProps<typeof Collapsible>, "children"> & {
  icon: LucideIcon;
  label: string;
  detail?: string;
  isStreaming: boolean;
  children: ReactNode;
  triggerClassName?: string;
  contentClassName?: string;
};

export function RuntimeProcessSection({
  className,
  contentClassName,
  detail,
  icon: Icon,
  isStreaming,
  label,
  onOpenChange,
  open,
  triggerClassName,
  children,
  ...props
}: RuntimeProcessSectionProps) {
  const startTimeRef = useRef<number | null>(isStreaming ? Date.now() : null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [hasDuration, setHasDuration] = useState(false);

  useEffect(() => {
    if (isStreaming) {
      if (startTimeRef.current === null) {
        startTimeRef.current = Date.now();
        setElapsedSeconds(0);
        setHasDuration(false);
      }

      const timer = window.setInterval(() => {
        const startAt = startTimeRef.current ?? Date.now();
        const duration = Math.max(
          0,
          Math.floor((Date.now() - startAt) / MS_IN_SECOND)
        );
        setElapsedSeconds(duration);
      }, MS_IN_SECOND);

      return () => window.clearInterval(timer);
    }

    if (startTimeRef.current !== null) {
      const duration = Math.max(
        0,
        Math.floor((Date.now() - startTimeRef.current) / MS_IN_SECOND)
      );
      setElapsedSeconds(duration);
      setHasDuration(true);
      startTimeRef.current = null;
    }
  }, [isStreaming]);

  return (
    <Collapsible
      className={cn("mb-0", className)}
      onOpenChange={onOpenChange}
      open={open}
      {...props}
    >
      <CollapsibleTrigger
        className={cn(
          "flex h-8 w-full items-center justify-start gap-1.5 px-2.5 py-1.5 text-left hover:text-foreground",
          triggerClassName
        )}
      >
        <Icon className="size-3.5 shrink-0 text-[var(--chat-runtime-text-secondary)]" />
        <span className="truncate text-xs font-medium leading-4 text-[var(--chat-runtime-text-secondary)]">
          {isStreaming ? (
            <Shimmer as="span" duration={1}>
              {label}
            </Shimmer>
          ) : (
            label
          )}
        </span>
        {detail ? (
          <span className="truncate text-[11px] leading-4 text-[var(--chat-runtime-text-secondary)]">
            {detail}
          </span>
        ) : null}
        {(isStreaming || hasDuration) && (
          <span className="shrink-0 text-[11px] leading-4 text-[var(--chat-runtime-text-secondary)]">
            {elapsedSeconds}s
          </span>
        )}
        <ChevronDownIcon
          className={cn(
            "size-3.5 shrink-0 text-[var(--chat-runtime-text-secondary)] transition-transform",
            open && "rotate-180"
          )}
        />
      </CollapsibleTrigger>
      <CollapsibleContent
        className={cn(COLLAPSIBLE_CONTENT_ANIMATION_CLASS, contentClassName)}
      >
        {children}
      </CollapsibleContent>
    </Collapsible>
  );
}
