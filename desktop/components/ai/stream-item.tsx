"use client";

import type { ComponentProps, ReactNode } from "react";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
import { AI_RUNTIME_DENSITY } from "@/components/ai/styles";
import { cn } from "@/lib/utils";
import { ChevronDownIcon } from "lucide-react";
import {
  createContext,
  memo,
  useContext,
  useEffect,
  useMemo,
  useRef,
} from "react";

type RunKey = string | number | undefined;

interface StreamItemContextValue {
  isOpen: boolean;
  isRunning: boolean;
  toggleFromUser: () => void;
}

const StreamItemContext = createContext<StreamItemContextValue | null>(null);

function useStreamItemContext() {
  const context = useContext(StreamItemContext);

  if (!context) {
    throw new Error("StreamItem components must be used within StreamItem");
  }

  return context;
}

export type StreamItemProps = ComponentProps<"div"> & {
  isRunning?: boolean;
  runKey?: string | number;
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  defaultOpenWhenRunning?: boolean;
  autoCloseOnFinish?: boolean;
  autoCloseDelayMs?: number;
};

export function StreamItem({
  children,
  className,
  isRunning = false,
  runKey,
  open,
  defaultOpen = false,
  onOpenChange,
  defaultOpenWhenRunning = true,
  autoCloseOnFinish = true,
  autoCloseDelayMs = 1_000,
  ...props
}: StreamItemProps) {
  const [isOpen, setIsOpen] = useControllableState({
    defaultProp: defaultOpen,
    onChange: onOpenChange,
    prop: open,
  });

  const currentRunKeyRef = useRef<RunKey>(runKey);
  const userCollapsedRunKeyRef = useRef<RunKey>(undefined);
  const runtimeOpenRunKeyRef = useRef<RunKey>(undefined);
  const autoCloseTimerRef = useRef<number | null>(null);
  const currentRunKey = runKey;

  useEffect(() => {
    if (currentRunKeyRef.current === currentRunKey) {
      return;
    }

    currentRunKeyRef.current = currentRunKey;
    userCollapsedRunKeyRef.current = undefined;
    runtimeOpenRunKeyRef.current =
      isOpen && isRunning && runtimeOpenRunKeyRef.current !== undefined
        ? currentRunKey
        : undefined;

    if (autoCloseTimerRef.current !== null) {
      window.clearTimeout(autoCloseTimerRef.current);
      autoCloseTimerRef.current = null;
    }
  }, [currentRunKey, isOpen, isRunning]);

  useEffect(() => {
    if (!isRunning || !defaultOpenWhenRunning || isOpen) {
      return;
    }

    if (
      userCollapsedRunKeyRef.current === currentRunKey ||
      runtimeOpenRunKeyRef.current === currentRunKey
    ) {
      return;
    }

    setIsOpen(true);
    runtimeOpenRunKeyRef.current = currentRunKey;
  }, [currentRunKey, defaultOpenWhenRunning, isOpen, isRunning, setIsOpen]);

  useEffect(() => {
    if (
      isRunning ||
      !autoCloseOnFinish ||
      !isOpen ||
      runtimeOpenRunKeyRef.current !== currentRunKey
    ) {
      return;
    }

    autoCloseTimerRef.current = window.setTimeout(() => {
      setIsOpen(false);
      runtimeOpenRunKeyRef.current = undefined;
      autoCloseTimerRef.current = null;
    }, autoCloseDelayMs);

    return () => {
      if (autoCloseTimerRef.current !== null) {
        window.clearTimeout(autoCloseTimerRef.current);
        autoCloseTimerRef.current = null;
      }
    };
  }, [
    autoCloseDelayMs,
    autoCloseOnFinish,
    currentRunKey,
    isOpen,
    isRunning,
    setIsOpen,
  ]);

  const contextValue = useMemo<StreamItemContextValue>(
    () => ({
      isOpen,
      isRunning,
      toggleFromUser: () => {
        const nextOpen = !isOpen;

        if (autoCloseTimerRef.current !== null) {
          window.clearTimeout(autoCloseTimerRef.current);
          autoCloseTimerRef.current = null;
        }

        if (!nextOpen && isRunning) {
          userCollapsedRunKeyRef.current = currentRunKey;
          runtimeOpenRunKeyRef.current = undefined;
        }

        if (nextOpen) {
          userCollapsedRunKeyRef.current = undefined;
          runtimeOpenRunKeyRef.current = undefined;
        }

        setIsOpen(nextOpen);
      },
    }),
    [currentRunKey, isOpen, isRunning, setIsOpen]
  );

  return (
    <StreamItemContext.Provider value={contextValue}>
      <div
        className={cn(
          "flex w-full flex-col",
          AI_RUNTIME_DENSITY.blockGap,
          className
        )}
        data-slot="stream-item"
        {...props}
      >
        {children}
      </div>
    </StreamItemContext.Provider>
  );
}

export type StreamItemTriggerProps = ComponentProps<"button"> & {
  icon?: ReactNode;
  label: ReactNode;
  status?: ReactNode;
};

export const StreamItemTrigger = memo(
  ({
    className,
    icon,
    label,
    onClick,
    status,
    type = "button",
    ...props
  }: StreamItemTriggerProps) => {
    const { isOpen, isRunning, toggleFromUser } = useStreamItemContext();

    return (
      <button
        aria-expanded={isOpen}
        className={cn(
          "group flex w-full items-center gap-1 rounded-sm py-0.5 text-left text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/30",
          AI_RUNTIME_DENSITY.triggerText,
          className
        )}
        data-slot="stream-item-trigger"
        onClick={(event) => {
          onClick?.(event);

          if (!event.defaultPrevented) {
            toggleFromUser();
          }
        }}
        type={type}
        {...props}
      >
        <span
          className={cn(
            "relative inline-flex min-w-0 items-center gap-1 overflow-hidden rounded-sm",
            isRunning && "text-foreground"
          )}
        >
          {isRunning ? (
            <span
              aria-hidden="true"
              className="pointer-events-none absolute inset-y-[-2px] -left-[30%] w-14 bg-linear-to-r from-transparent via-foreground/35 to-transparent animate-stream-item-shimmer"
              data-slot="stream-item-shimmer"
            />
          ) : null}
          {icon ? <span className="shrink-0">{icon}</span> : null}
          <span className="truncate">{label}</span>
        </span>
        {status ? (
          <span
            className={cn(
              "ml-auto shrink-0 text-muted-foreground",
              AI_RUNTIME_DENSITY.bodyText
            )}
          >
            {status}
          </span>
        ) : null}
        <ChevronDownIcon
          className={cn(
            "size-4 shrink-0 transition-transform",
            isOpen ? "rotate-180" : "rotate-0"
          )}
        />
      </button>
    );
  }
);

StreamItemTrigger.displayName = "StreamItemTrigger";

export type StreamItemContentProps = ComponentProps<"div">;

export function StreamItemContent({
  children,
  className,
  ...props
}: StreamItemContentProps) {
  const { isOpen } = useStreamItemContext();

  if (!isOpen) {
    return null;
  }

  return (
    <div
      className={cn(
        AI_RUNTIME_DENSITY.contentIndent,
        AI_RUNTIME_DENSITY.bodyText,
        "text-muted-foreground",
        className
      )}
      data-slot="stream-item-content"
      data-state="open"
      {...props}
    >
      {children}
    </div>
  );
}
