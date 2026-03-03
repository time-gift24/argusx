/**
 * @deprecated 已迁移到 components/ai/terminal.tsx
 * 请使用 `import { Terminal, TerminalContent, ... } from "@/components/ai/terminal"` 代替
 */
"use client";

import type { ComponentProps, HTMLAttributes } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import Ansi from "ansi-to-react";
import { CheckIcon, CopyIcon, TerminalIcon, Trash2Icon } from "lucide-react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { Shimmer } from "./shimmer";

interface TerminalContextType {
  output: string;
  isStreaming: boolean;
  autoScroll: boolean;
  compact: boolean;
  previewLines?: number;
  expanded: boolean;
  onToggleExpand?: (expanded: boolean) => void;
  onClear?: () => void;
}

const TerminalContext = createContext<TerminalContextType>({
  autoScroll: true,
  compact: false,
  expanded: true,
  isStreaming: false,
  output: "",
  previewLines: undefined,
});

export type TerminalProps = HTMLAttributes<HTMLDivElement> & {
  output: string;
  isStreaming?: boolean;
  autoScroll?: boolean;
  compact?: boolean;
  previewLines?: number;
  expanded?: boolean;
  onToggleExpand?: (expanded: boolean) => void;
  onClear?: () => void;
};

export const Terminal = ({
  output,
  isStreaming = false,
  autoScroll = true,
  compact = false,
  previewLines,
  expanded = true,
  onToggleExpand,
  onClear,
  className,
  children,
  ...props
}: TerminalProps) => {
  const hasCustomChildren = children !== undefined && children !== null;
  const contextValue = useMemo(
    () => ({
      autoScroll,
      compact,
      expanded,
      isStreaming,
      onClear,
      onToggleExpand,
      output,
      previewLines,
    }),
    [
      autoScroll,
      compact,
      expanded,
      isStreaming,
      onClear,
      onToggleExpand,
      output,
      previewLines,
    ]
  );

  return (
    <TerminalContext.Provider value={contextValue}>
      <div
        className={cn(
          "flex flex-col overflow-hidden",
          !hasCustomChildren &&
            "llm-chat-runtime-surface border bg-[var(--chat-runtime-surface-bg)] text-[var(--chat-runtime-surface-text)]",
          className
        )}
        {...props}
      >
        {children ?? (
          <>
            <TerminalHeader>
              <TerminalTitle />
              <div className="flex items-center gap-1">
                <TerminalStatus />
                <TerminalActions>
                  <TerminalCopyButton />
                  {onClear && <TerminalClearButton />}
                </TerminalActions>
              </div>
            </TerminalHeader>
            <TerminalContent />
          </>
        )}
      </div>
    </TerminalContext.Provider>
  );
};

export type TerminalHeaderProps = HTMLAttributes<HTMLDivElement>;

export const TerminalHeader = ({
  className,
  children,
  ...props
}: TerminalHeaderProps) => {
  const { compact } = useContext(TerminalContext);
  return (
    <div
      className={cn(
        "flex items-center justify-between",
        compact
          ? "px-2 pt-1.5 pb-0"
          : "px-2 pt-1.5 pb-0",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
};

export type TerminalTitleProps = HTMLAttributes<HTMLDivElement>;

export const TerminalTitle = ({
  className,
  children,
  ...props
}: TerminalTitleProps) => {
  const { compact } = useContext(TerminalContext);
  return (
    <div
      className={cn(
        "flex items-center gap-1.5 text-xs text-[var(--chat-runtime-surface-label)]",
        className
      )}
      {...props}
    >
      <TerminalIcon className={cn(compact ? "size-3.5" : "size-4")} />
      {children ?? "Terminal"}
    </div>
  );
};

export type TerminalStatusProps = HTMLAttributes<HTMLDivElement>;

export const TerminalStatus = ({
  className,
  children,
  ...props
}: TerminalStatusProps) => {
  const { compact, isStreaming } = useContext(TerminalContext);

  if (!isStreaming) {
    return null;
  }

  return (
    <div
      className={cn(
        "flex items-center gap-2",
        compact
          ? "text-[11px] text-[var(--chat-runtime-surface-label)]"
          : "text-[11px] text-[var(--chat-runtime-surface-label)]",
        className
      )}
      {...props}
    >
      {children ?? <Shimmer className={compact ? "w-12" : "w-16"} />}
    </div>
  );
};

export type TerminalActionsProps = HTMLAttributes<HTMLDivElement>;

export const TerminalActions = ({
  className,
  children,
  ...props
}: TerminalActionsProps) => (
  <div className={cn("flex items-center gap-1", className)} {...props}>
    {children}
  </div>
);

export type TerminalCopyButtonProps = ComponentProps<typeof Button> & {
  onCopy?: () => void;
  onError?: (error: Error) => void;
  timeout?: number;
};

export const TerminalCopyButton = ({
  onCopy,
  onError,
  timeout = 2000,
  children,
  className,
  ...props
}: TerminalCopyButtonProps) => {
  const [isCopied, setIsCopied] = useState(false);
  const timeoutRef = useRef<number>(0);
  const { output } = useContext(TerminalContext);

  const copyToClipboard = useCallback(async () => {
    if (typeof window === "undefined" || !navigator?.clipboard?.writeText) {
      onError?.(new Error("Clipboard API not available"));
      return;
    }

    try {
      await navigator.clipboard.writeText(output);
      setIsCopied(true);
      onCopy?.();
      timeoutRef.current = window.setTimeout(() => setIsCopied(false), timeout);
    } catch (error) {
      onError?.(error as Error);
    }
  }, [output, onCopy, onError, timeout]);

  useEffect(
    () => () => {
      window.clearTimeout(timeoutRef.current);
    },
    []
  );

  const Icon = isCopied ? CheckIcon : CopyIcon;

  return (
    <Button
      className={cn(
        "size-6 shrink-0 text-[var(--chat-runtime-surface-icon)] hover:bg-[var(--chat-runtime-surface-hover)] hover:text-[var(--chat-runtime-surface-text)]",
        className
      )}
      onClick={copyToClipboard}
      size="icon-sm"
      variant="ghost"
      {...props}
    >
      {children ?? <Icon size={14} />}
    </Button>
  );
};

export type TerminalClearButtonProps = ComponentProps<typeof Button>;

export const TerminalClearButton = ({
  children,
  className,
  ...props
}: TerminalClearButtonProps) => {
  const { onClear } = useContext(TerminalContext);

  if (!onClear) {
    return null;
  }

  return (
    <Button
      className={cn(
        "size-6 shrink-0 text-[var(--chat-runtime-surface-icon)] hover:bg-[var(--chat-runtime-surface-hover)] hover:text-[var(--chat-runtime-surface-text)]",
        className
      )}
      onClick={onClear}
      size="icon-sm"
      variant="ghost"
      {...props}
    >
      {children ?? <Trash2Icon size={14} />}
    </Button>
  );
};

export type TerminalContentProps = HTMLAttributes<HTMLDivElement>;

export const TerminalContent = ({
  className,
  children,
  ...props
}: TerminalContentProps) => {
  const {
    autoScroll,
    compact,
    expanded,
    isStreaming,
    onToggleExpand,
    output,
    previewLines,
  } = useContext(TerminalContext);
  const containerRef = useRef<HTMLDivElement>(null);

  const lines = output.split("\n");
  const hasPreview =
    typeof previewLines === "number" && previewLines > 0 && lines.length > previewLines;
  const shouldClamp = Boolean(hasPreview && !expanded);
  const visibleOutput = shouldClamp
    ? lines.slice(0, previewLines ?? 0).join("\n")
    : output;

  // biome-ignore lint/correctness/useExhaustiveDependencies: output triggers auto-scroll when new content arrives
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [visibleOutput, autoScroll]);

  return (
    <div
      className={cn(
        compact
          ? "max-h-56 overflow-auto px-2 pb-1.5 pt-0 font-mono text-[11px] leading-tight"
          : "max-h-72 overflow-auto px-2 pb-1.5 pt-0 font-mono text-xs leading-tight",
        className
      )}
      ref={containerRef}
      {...props}
    >
      {children ?? (
        <>
          <pre className="whitespace-pre-wrap break-words">
            <Ansi>{visibleOutput}</Ansi>
            {shouldClamp && "\n..."}
            {isStreaming && (
              <span
                className={cn(
                  "ml-0.5 inline-block animate-pulse",
                  compact
                    ? "h-4 w-1.5 bg-[var(--chat-runtime-surface-text)]"
                    : "h-4 w-1.5 bg-[var(--chat-runtime-surface-text)]"
                )}
              />
            )}
          </pre>
          {hasPreview && onToggleExpand && (
            <Button
              className="mt-1 h-6 px-2 text-[11px]"
              onClick={() => onToggleExpand(!expanded)}
              size="sm"
              type="button"
              variant="ghost"
            >
              {expanded ? "Show less" : `Show all (${lines.length} lines)`}
            </Button>
          )}
        </>
      )}
    </div>
  );
};
