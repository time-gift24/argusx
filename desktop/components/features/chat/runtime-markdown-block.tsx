"use client";

import type { BlockProps } from "streamdown";
import { Block } from "streamdown";

import { shouldHighlightFence } from "@/components/ai/highlight-policy";
import { RuntimeCodeSurface } from "./runtime-code-surface";

const TERMINAL_FENCE_LANGUAGES = new Set([
  "terminal",
  "shell-session",
  "bash",
  "zsh",
  "sh",
  "console",
]);

const COMPLETE_FENCED_CODE_BLOCK_PATTERN =
  /^[ \t]{0,3}(`{3,}|~{3,})([^\n]*)\n([\s\S]*?)\n?[ \t]{0,3}\1[ \t]*$/;
const INCOMPLETE_FENCED_CODE_BLOCK_PATTERN =
  /^[ \t]{0,3}(`{3,}|~{3,})([^\n]*)\n([\s\S]*)$/;

interface ParsedFencedCodeBlock {
  code: string;
  language: string;
}

const parseFencedCodeBlock = (
  content: string,
  isIncomplete: boolean
): ParsedFencedCodeBlock | null => {
  const normalized = content.replace(/\r\n?/g, "\n");
  const completeMatch = normalized.match(COMPLETE_FENCED_CODE_BLOCK_PATTERN);
  if (completeMatch) {
    const infoString = completeMatch[2]?.trim() ?? "";
    const language = infoString.split(/\s+/)[0]?.toLowerCase() ?? "";
    const code = completeMatch[3] ?? "";
    return { code, language };
  }

  if (!isIncomplete) {
    return null;
  }

  const match = normalized.match(INCOMPLETE_FENCED_CODE_BLOCK_PATTERN);
  if (!match) {
    return null;
  }

  const infoString = match[2]?.trim() ?? "";
  const language = infoString.split(/\s+/)[0]?.toLowerCase() ?? "";
  const code = match[3] ?? "";

  return { code, language };
};

export function RuntimeMarkdownBlock(props: BlockProps) {
  const parsed = parseFencedCodeBlock(props.content, props.isIncomplete);
  if (!parsed) {
    return <Block {...props} />;
  }

  if (parsed.language === "mermaid") {
    return <Block {...props} />;
  }

  const highlighted = shouldHighlightFence({
    isFenced: true,
    language: parsed.language,
  });

  if (TERMINAL_FENCE_LANGUAGES.has(parsed.language)) {
    return (
      <RuntimeCodeSurface
        code={parsed.code}
        isIncomplete={props.isIncomplete}
        language={parsed.language}
        highlighted={highlighted}
        mode="terminal"
      />
    );
  }

  return (
    <RuntimeCodeSurface
      code={parsed.code}
      isIncomplete={props.isIncomplete}
      language={parsed.language}
      highlighted={highlighted}
    />
  );
}
