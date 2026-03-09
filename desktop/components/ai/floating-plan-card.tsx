"use client";

import { type ComponentProps } from "react";

import { PlanQueue, type PlanSnapshot } from "@/components/ai/plan-queue";
import { cn } from "@/lib/utils";

export type FloatingPlanCardProps = ComponentProps<"div"> & {
  plan: PlanSnapshot;
};

export function FloatingPlanCard({
  className,
  plan,
  ...props
}: FloatingPlanCardProps) {
  return (
    <div
      className={cn(
        "pointer-events-auto mb-3 rounded-2xl border border-border/70 bg-background/90 p-3 shadow-sm backdrop-blur-sm",
        className
      )}
      data-slot="floating-plan-card"
      {...props}
    >
      <PlanQueue plan={plan} />
    </div>
  );
}
