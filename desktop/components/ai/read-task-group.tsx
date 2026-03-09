"use client";

import {
  Task,
  TaskContent,
  TaskItem,
  TaskItemFile,
  TaskTrigger,
} from "@/components/ai-elements/task";
import { AI_RUNTIME_DENSITY } from "@/components/ai/styles";
import { cn } from "@/lib/utils";

export type ReadTaskStatus =
  | "running"
  | "success"
  | "failed"
  | "timed_out"
  | "denied"
  | "cancelled";

export type ReadTaskItem = {
  callId: string;
  name: string;
  status: ReadTaskStatus;
  inputSummary?: string;
  outputSummary?: string;
  errorSummary?: string;
};

function formatStatus(status: ReadTaskStatus) {
  switch (status) {
    case "running":
      return "Running";
    case "success":
      return "Completed";
    case "failed":
      return "Failed";
    case "timed_out":
      return "Timed Out";
    case "denied":
      return "Denied";
    case "cancelled":
      return "Cancelled";
  }
}

export function ReadTaskGroup({ items }: { items: ReadTaskItem[] }) {
  if (items.length === 0) {
    return null;
  }

  return (
    <Task
      className="space-y-1"
      data-slot="read-task-group"
      defaultOpen={false}
    >
      <TaskTrigger title="Summary">
        <button
          className={cn(
            "flex w-full items-center gap-2 rounded-sm py-0.5 text-left text-muted-foreground transition-colors hover:text-foreground",
            AI_RUNTIME_DENSITY.triggerText
          )}
          type="button"
        >
          <span className="font-medium text-foreground">Summary</span>
          <span aria-hidden className="text-muted-foreground/80 text-xs">
            {items.length} items
          </span>
        </button>
      </TaskTrigger>
      <TaskContent className="mt-0">
        <div className="space-y-2">
          {items.map((item) => (
            <TaskItem
              className="space-y-1"
              data-slot="read-task-item"
              data-status={item.status}
              key={item.callId}
            >
              <div className="flex items-center gap-2">
                <span className="font-medium text-foreground text-sm">
                  {item.name}
                </span>
                <span className="text-muted-foreground text-xs">
                  {formatStatus(item.status)}
                </span>
              </div>
              {item.inputSummary ? (
                <TaskItemFile>{item.inputSummary}</TaskItemFile>
              ) : null}
              {item.outputSummary ? (
                <p className={cn(AI_RUNTIME_DENSITY.bodyText, "text-foreground/85")}>
                  {item.outputSummary}
                </p>
              ) : null}
              {item.errorSummary ? (
                <p className={cn(AI_RUNTIME_DENSITY.bodyText, "text-destructive")}>
                  {item.errorSummary}
                </p>
              ) : null}
            </TaskItem>
          ))}
        </div>
      </TaskContent>
    </Task>
  );
}
