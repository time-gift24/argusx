"use client";

import type { HTMLAttributes } from "react";
import type { BundledLanguage } from "shiki";

import { Shimmer } from "@/components/ai-elements/shimmer";
import { SURFACE_ICON_GHOST_BUTTON_CLASS } from "@/components/ai-elements/class-names";
import {
  CodeBlock,
  CodeBlockActions,
  CodeBlockCopyButton,
  CodeBlockFilename,
  CodeBlockHeader,
  CodeBlockTitle,
} from "@/components/ai-elements/code-block";
import {
  Terminal,
  TerminalContent,
  TerminalCopyButton,
} from "@/components/ai-elements/terminal";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { CheckIcon, CopyIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

const DEFAULT_PREVIEW_LINES = 8;

type RuntimeCodeSurfaceMode = "code" | "terminal";

export interface RuntimeCodeSurfaceProps
  extends HTMLAttributes<HTMLDivElement> {
  code: string;
  language?: string;
  isIncomplete?: boolean;
  highlighted?: boolean;
  mode?: RuntimeCodeSurfaceMode;
  previewLines?: number;
}

const countLines = (value: string): number => {
  if (!value) {
    return 0;
  }
  return value.split("\n").length;
};

const clampByLines = (value: string, maxLines: number): string =>
  value.split("\n").slice(0, maxLines).join("\n");

export function RuntimeCodeSurface({
  code,
  language,
  isIncomplete = false,
  highlighted = true,
  mode = "code",
  previewLines = DEFAULT_PREVIEW_LINES,
  className,
  ...props
}: RuntimeCodeSurfaceProps) {
  const [isCopied, setIsCopied] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const timeoutRef = useRef<number>(0);

  const totalLines = useMemo(() => countLines(code), [code]);
  const hasPreview = totalLines > previewLines;
  const shouldClamp = hasPreview && !expanded;
  const visibleCode = shouldClamp ? clampByLines(code, previewLines) : code;
  const hasCode = code.trim().length > 0;

  const copyToClipboard = useCallback(async () => {
    if (typeof window === "undefined" || !navigator?.clipboard?.writeText) {
      return;
    }

    try {
      await navigator.clipboard.writeText(code);
      setIsCopied(true);
      timeoutRef.current = window.setTimeout(() => setIsCopied(false), 2000);
    } catch {
      // Swallow clipboard failures for runtime-only UI interactions.
    }
  }, [code]);

  useEffect(
    () => () => {
      window.clearTimeout(timeoutRef.current);
    },
    []
  );

  useEffect(() => {
    if (!hasPreview) {
      setExpanded(false);
    }
  }, [hasPreview]);

  const CopyStateIcon = isCopied ? CheckIcon : CopyIcon;
  const copyLabel = mode === "terminal" ? "Copy terminal output" : "Copy code";

  if (!isIncomplete) {
    if (mode === "terminal") {
      return (
        <Terminal className="bg-transparent" compact output={code} {...props}>
          <div
            className={cn(
              "llm-chat-code-surface llm-chat-terminal-surface llm-chat-runtime-surface overflow-hidden",
              className
            )}
            data-highlighted={highlighted}
          >
            <CodeBlockHeader className="border-0 bg-transparent px-[var(--chat-runtime-code-padding-x)] pb-0 pt-[var(--chat-runtime-code-padding-y)]">
              <CodeBlockTitle className="text-[var(--chat-runtime-surface-label)]">
                <CodeBlockFilename className="[font-size:var(--chat-runtime-code-font-size)]">
                  terminal
                </CodeBlockFilename>
              </CodeBlockTitle>
              <CodeBlockActions className="-my-0.5 gap-1">
                <TerminalCopyButton
                  aria-label={copyLabel}
                  className={cn(
                    SURFACE_ICON_GHOST_BUTTON_CLASS,
                    "[&_svg]:size-[14px]"
                  )}
                  disabled={!hasCode}
                  title={copyLabel}
                />
              </CodeBlockActions>
            </CodeBlockHeader>
            <TerminalContent className="!max-h-56 !bg-transparent !px-[var(--chat-runtime-code-padding-x)] !pb-[var(--chat-runtime-code-padding-y)] !pt-0 ![font-size:var(--chat-runtime-code-font-size)] ![line-height:var(--chat-runtime-code-line-height)]" />
          </div>
        </Terminal>
      );
    }

    // Code mode: use syntax highlighting only when highlighted=true
    if (!highlighted) {
      return (
        <div
          className={cn(
            "llm-chat-code-surface llm-chat-runtime-surface",
            className
          )}
          data-highlighted="false"
          {...props}
        >
          <CodeBlockHeader>
            <CodeBlockTitle>
              <CodeBlockFilename>{language || "code"}</CodeBlockFilename>
            </CodeBlockTitle>
            <CodeBlockActions>
              <Button
                aria-label={copyLabel}
                className={cn(
                  SURFACE_ICON_GHOST_BUTTON_CLASS,
                  "[&_svg]:size-[14px]"
                )}
                disabled={!hasCode}
                onClick={copyToClipboard}
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <CopyStateIcon size={14} />
              </Button>
            </CodeBlockActions>
          </CodeBlockHeader>
          <div className="px-[var(--chat-runtime-code-padding-x)] pb-[var(--chat-runtime-code-padding-y)] font-mono [font-size:var(--chat-runtime-code-font-size)] [line-height:var(--chat-runtime-code-line-height)]">
            <pre className="m-0 whitespace-pre-wrap break-words">
              <code>{code}</code>
            </pre>
          </div>
        </div>
      );
    }

    return (
      <div data-highlighted="true">
        <CodeBlock
          className={cn("llm-chat-code-surface", className)}
          code={code}
          compact
          language={(language || "text") as BundledLanguage}
          showLineNumbers={false}
          {...props}
        >
          <CodeBlockHeader>
            <CodeBlockTitle>
              <CodeBlockFilename>{language || "text"}</CodeBlockFilename>
            </CodeBlockTitle>
            <CodeBlockActions>
              <CodeBlockCopyButton
                aria-label={copyLabel}
                className={cn(
                  SURFACE_ICON_GHOST_BUTTON_CLASS,
                  "[&_svg]:size-[14px]"
                )}
                disabled={!hasCode}
                title={copyLabel}
              />
            </CodeBlockActions>
          </CodeBlockHeader>
        </CodeBlock>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "llm-chat-code-surface llm-chat-runtime-surface relative overflow-hidden",
        mode === "terminal" && "llm-chat-terminal-surface",
        className
      )}
      {...props}
    >
      <div className="pointer-events-none absolute right-[var(--chat-runtime-code-padding-x)] top-[var(--chat-runtime-code-padding-y)] z-10">
        <Button
          aria-label={copyLabel}
          className={cn(
            SURFACE_ICON_GHOST_BUTTON_CLASS,
            "pointer-events-auto border-0 shadow-none"
          )}
          disabled={!hasCode}
          onClick={copyToClipboard}
          size="icon-sm"
          type="button"
          variant="ghost"
        >
          <CopyStateIcon size={14} />
        </Button>
      </div>

      {isIncomplete ? (
        <div className="flex items-center gap-2 px-[var(--chat-runtime-code-padding-x)] pb-0 pt-[var(--chat-runtime-code-padding-y)] text-[var(--chat-runtime-surface-label)] [font-size:var(--chat-runtime-code-font-size)]">
          <span>生成中</span>
          <Shimmer className="h-3 w-10 rounded-sm" />
        </div>
      ) : null}

      <div
        className={cn(
          "max-h-56 overflow-auto px-[var(--chat-runtime-code-padding-x)] pb-[var(--chat-runtime-code-padding-y)] font-mono [font-size:var(--chat-runtime-code-font-size)] [line-height:var(--chat-runtime-code-line-height)]",
          isIncomplete ? "pt-1" : "pt-[var(--chat-runtime-code-padding-y)]"
        )}
      >
        <pre className="m-0 whitespace-pre-wrap break-words pr-8">
          <code>
            {visibleCode}
            {shouldClamp ? "\n..." : ""}
          </code>
        </pre>
      </div>

      {hasPreview ? (
        <div className="px-[var(--chat-runtime-code-padding-x)] pb-[var(--chat-runtime-code-padding-y)] pt-0">
          <Button
            className="h-6 px-2 text-[11px]"
            onClick={() => setExpanded((current) => !current)}
            size="sm"
            type="button"
            variant="ghost"
          >
            {expanded ? "收起" : `展开 (${totalLines} 行)`}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
