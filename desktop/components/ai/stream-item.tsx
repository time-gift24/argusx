"use client";

import type { ComponentProps, ReactNode } from "react";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
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
  const autoOpenedRunKeyRef = useRef<RunKey>(undefined);
  const autoCloseTimerRef = useRef<number | null>(null);
  const currentRunKey = runKey;

  useEffect(() => {
    if (currentRunKeyRef.current === currentRunKey) {
      return;
    }

    currentRunKeyRef.current = currentRunKey;
    userCollapsedRunKeyRef.current = undefined;
    autoOpenedRunKeyRef.current = undefined;

    if (autoCloseTimerRef.current !== null) {
      window.clearTimeout(autoCloseTimerRef.current);
      autoCloseTimerRef.current = null;
    }
  }, [currentRunKey]);

  useEffect(() => {
    if (!isRunning || !defaultOpenWhenRunning || isOpen) {
      return;
    }

    if (userCollapsedRunKeyRef.current === currentRunKey) {
      return;
    }

    setIsOpen(true);
    autoOpenedRunKeyRef.current = currentRunKey;
  }, [currentRunKey, defaultOpenWhenRunning, isOpen, isRunning, setIsOpen]);

  useEffect(() => {
    if (
      isRunning ||
      !autoCloseOnFinish ||
      !isOpen ||
      autoOpenedRunKeyRef.current !== currentRunKey
    ) {
      return;
    }

    autoCloseTimerRef.current = window.setTimeout(() => {
      setIsOpen(false);
      autoOpenedRunKeyRef.current = undefined;
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
        }

        if (nextOpen) {
          userCollapsedRunKeyRef.current = undefined;
          if (!isRunning) {
            autoOpenedRunKeyRef.current = undefined;
          }
        }

        setIsOpen(nextOpen);
      },
    }),
    [currentRunKey, isOpen, isRunning, setIsOpen]
  );

  return (
    <StreamItemContext.Provider value={contextValue}>
      <div
        className={cn("flex w-full flex-col gap-2", className)}
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
          "group flex w-full items-center gap-2 rounded-sm py-0.5 text-left text-sm text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/30",
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
            "relative inline-flex min-w-0 items-center gap-2 overflow-hidden rounded-sm",
            isRunning &&
              "text-foreground after:pointer-events-none after:absolute after:inset-y-[-2px] after:left-[-30%] after:w-12 after:bg-linear-to-r after:from-transparent after:via-foreground/35 after:to-transparent after:animate-stream-item-shimmer"
          )}
        >
          {icon ? <span className="shrink-0">{icon}</span> : null}
          <span className="truncate">{label}</span>
        </span>
        {status ? (
          <span className="ml-auto shrink-0 text-[11px] text-muted-foreground">
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
      className={cn("pl-6 text-sm text-muted-foreground", className)}
      data-slot="stream-item-content"
      data-state="open"
      {...props}
    >
      {children}
    </div>
  );
}
