"use client";

import {
  Queue,
  QueueItem,
  QueueItemContent,
  QueueItemIndicator,
  QueueSection,
  QueueSectionContent,
  QueueSectionTrigger,
} from "@/components/ai-elements/queue";
import { cn } from "@/lib/utils";

export type PlanTaskStatus = "pending" | "in_progress" | "completed";

export type PlanTask = {
  id: string;
  title: string;
  status: PlanTaskStatus;
};

export type PlanSnapshot = {
  title: string;
  description?: string | null;
  isStreaming: boolean;
  sourceCallId: string;
  tasks: PlanTask[];
};

export function PlanQueue({ plan }: { plan: PlanSnapshot }) {
  return (
    <Queue
      className="gap-1.5"
      data-slot="plan-queue"
      data-streaming={plan.isStreaming ? "true" : "false"}
    >
      <QueueSection defaultOpen>
        <QueueSectionTrigger className="items-start">
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm text-foreground">{plan.title}</p>
            {plan.description ? (
              <p className="mt-0.5 text-xs text-muted-foreground">
                {plan.description}
              </p>
            ) : null}
          </div>
        </QueueSectionTrigger>
        <QueueSectionContent>
          <div className="pt-1">
            {plan.tasks.map((task) => {
              const isCompleted = task.status === "completed";
              const isInProgress = task.status === "in_progress";

              return (
                <QueueItem
                  className="px-2.5"
                  data-slot="plan-queue-item"
                  data-status={task.status}
                  key={task.id}
                >
                  <div className="flex items-center gap-2">
                    <QueueItemIndicator
                      className={cn(
                        isInProgress
                          ? "border-primary bg-primary/20"
                          : undefined
                      )}
                      completed={isCompleted}
                    />
                    <QueueItemContent
                      className={cn(
                        isInProgress ? "text-foreground" : undefined
                      )}
                      completed={isCompleted}
                    >
                      {task.title}
                    </QueueItemContent>
                  </div>
                </QueueItem>
              );
            })}
          </div>
        </QueueSectionContent>
      </QueueSection>
    </Queue>
  );
}
