"use client";

import type { ComponentProps } from "react";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { ChevronDownIcon, SearchIcon } from "lucide-react";

import { COLLAPSIBLE_CONTENT_ANIMATION_CLASS } from "./class-names";

export type TaskItemFileProps = ComponentProps<"div">;

export const TaskItemFile = ({
  children,
  className,
  ...props
}: TaskItemFileProps) => (
  <div
    className={cn(
      "inline-flex items-center gap-1 rounded-md border bg-secondary px-1.5 py-0.5 text-foreground text-xs",
      className
    )}
    {...props}
  >
    {children}
  </div>
);

export type TaskItemProps = ComponentProps<"div"> & {
  compact?: boolean;
};

export const TaskItem = ({
  children,
  className,
  compact = false,
  ...props
}: TaskItemProps) => (
  <div
    className={cn(
      compact ? "text-[12px] leading-5 text-muted-foreground" : "text-muted-foreground text-sm",
      className
    )}
    {...props}
  >
    {children}
  </div>
);

export type TaskProps = ComponentProps<typeof Collapsible> & {
  compact?: boolean;
};

export const Task = ({
  defaultOpen = true,
  className,
  compact = false,
  ...props
}: TaskProps) => (
  <Collapsible
    className={cn(compact && "space-y-1", className)}
    defaultOpen={defaultOpen}
    {...props}
  />
);

export type TaskTriggerProps = ComponentProps<typeof CollapsibleTrigger> & {
  title: string;
  compact?: boolean;
};

export const TaskTrigger = ({
  children,
  className,
  title,
  compact = false,
  ...props
}: TaskTriggerProps) => (
  <CollapsibleTrigger asChild className={cn("group", className)} {...props}>
    {children ?? (
      <div
        className={cn(
          "flex w-full cursor-pointer items-center gap-2 text-muted-foreground transition-colors hover:text-foreground",
          compact ? "text-xs" : "text-sm"
        )}
      >
        <SearchIcon className={cn(compact ? "size-3.5" : "size-4")} />
        <p className={cn(compact ? "text-xs" : "text-sm")}>{title}</p>
        <ChevronDownIcon
          className={cn(
            compact ? "size-3.5" : "size-4",
            "transition-transform group-data-[state=open]:rotate-180"
          )}
        />
      </div>
    )}
  </CollapsibleTrigger>
);

export type TaskContentProps = ComponentProps<typeof CollapsibleContent> & {
  compact?: boolean;
};

export const TaskContent = ({
  children,
  className,
  compact = false,
  ...props
}: TaskContentProps) => (
  <CollapsibleContent
    className={cn(
      COLLAPSIBLE_CONTENT_ANIMATION_CLASS,
      "text-popover-foreground",
      className
    )}
    {...props}
  >
    <div
      className={cn(
        compact ? "mt-2 space-y-1.5 border-muted border-l-2 pl-3" : "mt-4 space-y-2 border-muted border-l-2 pl-4"
      )}
    >
      {children}
    </div>
  </CollapsibleContent>
);
