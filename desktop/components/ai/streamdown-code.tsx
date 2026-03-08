"use client";

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */

import type { Components, ExtraProps } from "streamdown";
import type { HighlightResult } from "@streamdown/code";
import type { BundledLanguage } from "streamdown";
import type { ComponentProps, CSSProperties, ReactNode } from "react";

import {
  BinaryIcon,
  BracesIcon,
  ChevronDownIcon,
  Code2Icon,
  DatabaseIcon,
  FileCode2Icon,
  FileJsonIcon,
  GlobeIcon,
  SigmaIcon,
  SquareFunctionIcon,
  TerminalIcon,
  WorkflowIcon,
} from "lucide-react";
import {
  CodeBlockContainer,
  CodeBlockCopyButton,
  CodeBlockDownloadButton,
  Streamdown,
  useIsCodeFenceIncomplete,
} from "streamdown";
import { useEffect, useMemo, useState } from "react";

import { sharedCodePlugin } from "@/components/ai/shared-code-highlighter";
import {
  StreamItem,
  StreamItemTrigger,
  useStreamItemState,
  StreamItemViewport,
} from "@/components/ai/stream-item";
import {
  sharedStreamdownClassName,
  sharedStreamdownControls,
  sharedStreamdownIcons,
  sharedStreamdownPlugins,
  sharedStreamdownTranslations,
} from "@/components/ai/streamdown-config";
import { AI_RUNTIME_DENSITY } from "@/components/ai/styles";
import { cn } from "@/lib/utils";

const LANGUAGE_CLASSNAME_PATTERN = /language-([A-Za-z0-9_-]+)/;
const START_LINE_PATTERN = /(?:^|\s)startLine=(\d+)(?:\s|$)/;
const DEFAULT_COLLAPSED_CODE_LINES = 6;

const DISPLAY_LANGUAGE_BY_TOKEN: Record<string, string> = {
  js: "JavaScript",
  jsx: "JSX",
  sh: "Shell",
  ts: "TypeScript",
  tsx: "TSX",
};

const CODE_LINE_CLASSNAME =
  "block before:content-[counter(line)] before:inline-block before:[counter-increment:line] before:w-6 before:mr-4 before:text-[13px] before:text-right before:text-muted-foreground/50 before:font-mono before:select-none";

const LANGUAGE_ICON_BY_TOKEN = {
  bash: TerminalIcon,
  css: BracesIcon,
  go: WorkflowIcon,
  html: GlobeIcon,
  javascript: BracesIcon,
  js: BracesIcon,
  json: FileJsonIcon,
  jsx: FileCode2Icon,
  py: SquareFunctionIcon,
  python: SquareFunctionIcon,
  rs: BinaryIcon,
  rust: BinaryIcon,
  sh: TerminalIcon,
  shell: TerminalIcon,
  sql: DatabaseIcon,
  tex: SigmaIcon,
  toml: FileJsonIcon,
  ts: FileCode2Icon,
  tsx: FileCode2Icon,
  typescript: FileCode2Icon,
  xml: Code2Icon,
  yaml: WorkflowIcon,
  yml: WorkflowIcon,
  zsh: TerminalIcon,
} as const;

type StreamdownCodeProps = ComponentProps<"code"> &
  ExtraProps & {
    "data-block"?: string;
  };

type CodeTokens = HighlightResult;
type CodeToken = CodeTokens["tokens"][number][number];

function extractTextContent(children: ReactNode): string {
  if (typeof children === "string") {
    return children;
  }

  if (Array.isArray(children)) {
    return children.map(extractTextContent).join("");
  }

  if (
    children &&
    typeof children === "object" &&
    "props" in children &&
    children.props &&
    typeof children.props === "object" &&
    "children" in children.props
  ) {
    return extractTextContent(children.props.children as ReactNode);
  }

  return "";
}

function getLanguageToken(className?: string) {
  return className?.match(LANGUAGE_CLASSNAME_PATTERN)?.[1]?.toLowerCase() ?? "";
}

function getLanguageLabel(languageToken: string) {
  if (!languageToken) {
    return "Code";
  }

  return (
    DISPLAY_LANGUAGE_BY_TOKEN[languageToken] ??
    languageToken
      .replace(/[-_]+/g, " ")
      .replace(/\b\w/g, (character) => character.toUpperCase())
  );
}

function getLanguageIcon(languageToken: string) {
  return LANGUAGE_ICON_BY_TOKEN[
    languageToken as keyof typeof LANGUAGE_ICON_BY_TOKEN
  ] ?? Code2Icon;
}

function createPlainTokens(code: string): CodeTokens {
  return {
    bg: "transparent",
    fg: "inherit",
    tokens: code.split("\n").map((line) =>
      line === ""
        ? []
        : [
            {
              bgColor: "transparent",
              color: "inherit",
              content: line,
              htmlStyle: {},
              offset: 0,
            } satisfies CodeToken,
          ]
    ),
  };
}

function getSupportedLanguage(languageToken: string) {
  if (!languageToken) {
    return null;
  }

  return sharedCodePlugin.supportsLanguage(languageToken as BundledLanguage)
    ? (languageToken as BundledLanguage)
    : null;
}

function renderTokenStyles(token: CodeToken): CSSProperties {
  return {
    backgroundColor: "transparent",
    color: token.color,
    ...token.htmlStyle,
  };
}

function useCodeTokens(code: string, languageToken: string) {
  const supportedLanguage = getSupportedLanguage(languageToken);
  const rawTokens = useMemo(() => createPlainTokens(code), [code]);
  const [tokenized, setTokenized] = useState<CodeTokens>(rawTokens);

  useEffect(() => {
    if (!supportedLanguage) {
      setTokenized(rawTokens);
      return;
    }

    let cancelled = false;
    const highlighted =
      sharedCodePlugin.highlight(
        {
          code,
          language: supportedLanguage,
          themes: sharedCodePlugin.getThemes(),
        },
        (result) => {
          if (!cancelled) {
            setTokenized(result);
          }
        }
      ) ?? rawTokens;

    setTokenized(highlighted);

    return () => {
      cancelled = true;
    };
  }, [code, rawTokens, supportedLanguage]);

  return tokenized;
}

function getStartLine(node?: ExtraProps["node"]) {
  const metastring = node?.properties?.metastring;

  if (typeof metastring !== "string") {
    return undefined;
  }

  const match = metastring.match(START_LINE_PATTERN);

  if (!match) {
    return undefined;
  }

  const parsed = Number.parseInt(match[1], 10);

  return Number.isNaN(parsed) || parsed < 1 ? undefined : parsed;
}

function MermaidFence({ chart }: { chart: string }) {
  return (
    <Streamdown
      className={sharedStreamdownClassName}
      controls={sharedStreamdownControls}
      icons={sharedStreamdownIcons}
      plugins={sharedStreamdownPlugins}
      translations={sharedStreamdownTranslations}
    >
      {["```mermaid", chart.replace(/\n$/, ""), "```"].join("\n")}
    </Streamdown>
  );
}

function InlineCode({
  children,
  className,
  ...props
}: ComponentProps<"code">) {
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

function CodeExpandHint({ canExpand }: { canExpand: boolean }) {
  const { isOpen, toggleFromUser } = useStreamItemState();

  if (!canExpand || isOpen) {
    return null;
  }

  return (
    <div data-streamdown="code-expand-hint">
      <button
        aria-label="Expand code"
        data-streamdown="code-expand-button"
        onClick={toggleFromUser}
        type="button"
      >
        <ChevronDownIcon aria-hidden="true" className="size-[10px] shrink-0" />
        <span>Expand</span>
      </button>
    </div>
  );
}

export function StreamdownCode({
  children,
  className,
  node,
  ...props
}: StreamdownCodeProps) {
  const isIncomplete = useIsCodeFenceIncomplete();
  const isBlockCode = "data-block" in props;

  if (!isBlockCode) {
    return <InlineCode className={className} {...props}>{children}</InlineCode>;
  }

  const rawCode = extractTextContent(children);
  const code = rawCode.replace(/\n+$/, "");
  const languageToken = getLanguageToken(className);
  const isCodeRunning = isIncomplete || !rawCode.endsWith("\n");

  if (languageToken === "mermaid") {
    return <MermaidFence chart={code} />;
  }

  const languageLabel = getLanguageLabel(languageToken);
  const languageIconToken = languageToken || "text";
  const LanguageIcon = getLanguageIcon(languageToken);
  const isExpandable =
    code === "" ? false : code.split("\n").length > DEFAULT_COLLAPSED_CODE_LINES;
  const startLine = getStartLine(node);
  const tokenized = useCodeTokens(code, languageToken);

  return (
    <StreamItem
      autoCloseOnFinish={false}
      className="w-full"
      defaultOpen={false}
      defaultOpenWhenRunning={false}
      isRunning={isCodeRunning}
    >
      <StreamItemTrigger
        className="text-foreground/80 hover:text-foreground"
        icon={
          <span
            className="flex items-center justify-center"
            data-language={languageIconToken}
            data-streamdown="code-language-icon"
          >
            <LanguageIcon className="size-[10px]" />
          </span>
        }
        label={
          <span
            className={cn(
              AI_RUNTIME_DENSITY.triggerText,
              "font-mono tracking-[0.02em] text-current"
            )}
          >
            {languageLabel}
          </span>
        }
        status={isCodeRunning ? "Running" : "Ready"}
      />
      <StreamItemViewport className="w-full pl-0">
        <div className="group relative w-full" data-streamdown="custom-code-panel">
          <CodeBlockContainer
            className="m-0"
            isIncomplete={isCodeRunning}
            language={languageToken || "text"}
          >
            <div data-streamdown="code-block-body">
              <div data-streamdown="code-block-actions">
                <CodeBlockCopyButton code={code} />
                <CodeBlockDownloadButton
                  code={code}
                  language={languageToken || undefined}
                />
              </div>
              <pre
                style={{
                  backgroundColor: "transparent",
                  color: tokenized.fg,
                }}
              >
                <code
                  className="[counter-increment:line_0] [counter-reset:line]"
                  style={
                    startLine && startLine > 1
                      ? { counterReset: `line ${startLine - 1}` }
                      : undefined
                  }
                >
                  {tokenized.tokens.map((line, lineIndex) => (
                    <span className={CODE_LINE_CLASSNAME} key={`code-line-${lineIndex}`}>
                      {line.length === 0
                        ? "\n"
                        : line.map((token, tokenIndex) => (
                            <span
                              key={`code-line-${lineIndex}-token-${tokenIndex}`}
                              style={renderTokenStyles(token)}
                            >
                              {token.content}
                            </span>
                          ))}
                    </span>
                  ))}
                </code>
              </pre>
            </div>
          </CodeBlockContainer>
          <CodeExpandHint canExpand={isExpandable} />
        </div>
      </StreamItemViewport>
    </StreamItem>
  );
}

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
export const sharedStreamdownComponents = {
  code: StreamdownCode,
} satisfies Components;
