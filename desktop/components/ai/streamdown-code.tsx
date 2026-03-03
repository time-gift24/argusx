"use client";

import type { ExtraProps } from "streamdown";
import { useIsCodeFenceIncomplete } from "streamdown";

import { shouldHighlightFence } from "@/components/ai/highlight-policy";
import { RuntimeCodeSurface } from "@/components/features/chat/runtime-code-surface";
import { cn } from "@/lib/utils";

const TERMINAL_FENCE_LANGUAGES = new Set([
  "terminal",
  "shell-session",
  "bash",
  "zsh",
  "sh",
  "console",
]);

/**
 * Custom code component for Streamdown that handles both inline code and code blocks.
 *
 * This replaces the default double-rendering behavior where Streamdown's default
 * code rendering was nested inside RuntimeMarkdownBlock's RuntimeCodeSurface.
 *
 * Detection logic:
 * - Uses official useIsCodeFenceIncomplete hook for streaming code blocks
 * - Code blocks have className="language-xxx" from Streamdown's rehype processing
 * - Inline code has no className or different styling
 */
export function StreamdownCode({
  children,
  className,
  ...props
}: React.HTMLAttributes<HTMLElement> & ExtraProps) {
  // Official Streamdown hook: detects incomplete code fences during streaming
  const isIncompleteCodeFence = useIsCodeFenceIncomplete();

  // Check if this is a code block by looking for language-xxx class
  // Streamdown adds className like "language-typescript" or "language-python"
  const hasLanguageClass = className?.includes("language-") ?? false;

  // Code block if: has language class OR is incomplete code fence (streaming)
  const isCodeBlock = hasLanguageClass || isIncompleteCodeFence;

  if (!isCodeBlock) {
    // Inline code - simple styling with subtle background
    return (
      <code
        className={cn(
          "rounded bg-muted px-1.5 py-0.5 font-mono text-sm",
          className
        )}
        data-streamdown="inline-code"
        {...props}
      >
        {children}
      </code>
    );
  }

  // Block code - extract language from className
  const languageMatch = className?.match(/language-(\w+)/);
  const language = languageMatch?.[1] || "text";

  // Children should be the code content as string
  const code =
    typeof children === "string"
      ? children
      : children instanceof Array
        ? children.join("")
        : String(children ?? "");

  // Determine if this is a terminal language
  const isTerminal = TERMINAL_FENCE_LANGUAGES.has(language);

  // Apply deterministic highlight policy
  const highlighted = shouldHighlightFence({
    isFenced: true,
    language,
  });

  return (
    <RuntimeCodeSurface
      code={code}
      language={language}
      highlighted={highlighted}
      mode={isTerminal ? "terminal" : "code"}
      className="my-2"
    />
  );
}
