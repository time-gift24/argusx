"use client";

import type { HTMLAttributes } from "react";

import { RuntimeCodeSurface } from "@/components/features/chat/runtime-code-surface";
import { cn } from "@/lib/utils";

interface StreamdownCodeProps extends HTMLAttributes<HTMLElement> {
  children?: React.ReactNode;
}

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
 * - Code blocks have className="language-xxx" from Streamdown's rehype processing
 * - Inline code has no className or different styling
 */
export function StreamdownCode({
  children,
  className,
  ...props
}: StreamdownCodeProps) {
  // Check if this is a code block by looking for language-xxx class
  // Streamdown adds className like "language-typescript" or "language-python"
  const isCodeBlock = className?.includes("language-") ?? false;

  if (!isCodeBlock) {
    // Inline code - simple styling with subtle background
    return (
      <code
        className={cn(
          "rounded bg-muted px-1.5 py-0.5 font-mono text-sm",
          className
        )}
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

  return (
    <RuntimeCodeSurface
      code={code}
      language={language}
      mode={isTerminal ? "terminal" : "code"}
      className="my-2"
    />
  );
}
