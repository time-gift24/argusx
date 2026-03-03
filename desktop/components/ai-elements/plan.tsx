/**
 * @deprecated 已迁移到 components/ai/plan.tsx
 * 请使用 `import { Plan, PlanContent, ... } from "@/components/ai/plan"` 代替
 */
"use client";

import type { ComponentProps } from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { ChevronsUpDownIcon } from "lucide-react";
import { createContext, useContext } from "react";

import { Shimmer } from "./shimmer";

interface PlanContextValue {
  isStreaming: boolean;
  compact: boolean;
}

const PlanContext = createContext<PlanContextValue | null>(null);

const usePlan = () => {
  const context = useContext(PlanContext);
  if (!context) {
    throw new Error("Plan components must be used within Plan");
  }
  return context;
};

export type PlanProps = ComponentProps<typeof Collapsible> & {
  isStreaming?: boolean;
  compact?: boolean;
};

export const Plan = ({
  className,
  isStreaming = false,
  compact = false,
  children,
  ...props
}: PlanProps) => (
  <PlanContext.Provider value={{ compact, isStreaming }}>
    <Collapsible asChild data-slot="plan" {...props}>
      <Card
        className={cn(
          "shadow-none",
          compact && "rounded-md border-border/60 py-2.5",
          className
        )}
        size={compact ? "sm" : "default"}
      >
        {children}
      </Card>
    </Collapsible>
  </PlanContext.Provider>
);

export type PlanHeaderProps = ComponentProps<typeof CardHeader>;

export const PlanHeader = ({ className, ...props }: PlanHeaderProps) => {
  const { compact } = usePlan();
  return (
    <CardHeader
      className={cn(
        "flex items-start justify-between",
        compact && "gap-1 px-2.5 py-1.5",
        className
      )}
      data-slot="plan-header"
      {...props}
    />
  );
};

export type PlanTitleProps = Omit<
  ComponentProps<typeof CardTitle>,
  "children"
> & {
  children: string;
};

export const PlanTitle = ({ children, ...props }: PlanTitleProps) => {
  const { compact, isStreaming } = usePlan();

  return (
    <CardTitle
      className={cn(compact && "text-xs leading-4")}
      data-slot="plan-title"
      {...props}
    >
      {isStreaming ? <Shimmer>{children}</Shimmer> : children}
    </CardTitle>
  );
};

export type PlanDescriptionProps = Omit<
  ComponentProps<typeof CardDescription>,
  "children"
> & {
  children: string;
};

export const PlanDescription = ({
  className,
  children,
  ...props
}: PlanDescriptionProps) => {
  const { compact, isStreaming } = usePlan();

  return (
    <CardDescription
      className={cn(
        "text-balance",
        compact && "text-[11px] leading-4",
        className
      )}
      data-slot="plan-description"
      {...props}
    >
      {isStreaming ? <Shimmer>{children}</Shimmer> : children}
    </CardDescription>
  );
};

export type PlanActionProps = ComponentProps<typeof CardAction>;

export const PlanAction = (props: PlanActionProps) => (
  <CardAction data-slot="plan-action" {...props} />
);

export type PlanContentProps = ComponentProps<typeof CardContent>;

export const PlanContent = ({ className, ...props }: PlanContentProps) => {
  const { compact } = usePlan();
  return (
    <CollapsibleContent asChild>
      <CardContent
        className={cn(compact && "px-2.5 pb-1.5 pt-0", className)}
        data-slot="plan-content"
        {...props}
      />
    </CollapsibleContent>
  );
};

export type PlanFooterProps = ComponentProps<"div">;

export const PlanFooter = (props: PlanFooterProps) => (
  <CardFooter data-slot="plan-footer" {...props} />
);

export type PlanTriggerProps = ComponentProps<typeof CollapsibleTrigger>;

export const PlanTrigger = ({
  compact,
  className,
  ...props
}: PlanTriggerProps & { compact?: boolean }) => {
  const { compact: compactFromContext } = usePlan();
  const isCompact = compact ?? compactFromContext;
  return (
    <CollapsibleTrigger asChild>
      <Button
        className={cn(isCompact ? "size-6" : "size-8", className)}
        data-slot="plan-trigger"
        size={isCompact ? "icon-sm" : "icon"}
        variant="ghost"
        {...props}
      >
        <ChevronsUpDownIcon className={cn(isCompact ? "size-3.5" : "size-4")} />
        <span className="sr-only">Toggle plan</span>
      </Button>
    </CollapsibleTrigger>
  );
};
