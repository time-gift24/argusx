"use client";

import type { ComponentProps, CSSProperties, ReactNode } from "react";
import type {
  HighlightResult,
} from "@streamdown/code";
import type { BundledLanguage } from "streamdown";

import { CheckIcon, CopyIcon, DownloadIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { sharedCodePlugin } from "@/components/ai/shared-code-highlighter";
import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
  type StreamItemProps,
} from "@/components/ai/stream-item";
import {
  AI_CODE_SURFACE_CLASSNAME,
  AI_RUNTIME_DENSITY,
} from "@/components/ai/styles";
import { cn } from "@/lib/utils";

const DOWNLOAD_EXTENSION_BY_LANGUAGE: Record<string, string> = {
  bash: "sh",
  javascript: "js",
  js: "js",
  json: "json",
  jsx: "jsx",
  sh: "sh",
  shell: "sh",
  ts: "ts",
  tsx: "tsx",
  typescript: "ts",
};

type RuntimeTokens = HighlightResult;
type RuntimeToken = RuntimeTokens["tokens"][number][number];

export type RuntimeCodeSection = {
  code: string;
  downloadFilename?: string;
  id: string;
  label: string;
  language?: string;
  tone?: "default" | "error";
};

export type RuntimeCodePanelProps = Omit<StreamItemProps, "children"> & {
  icon?: ReactNode;
  sections: RuntimeCodeSection[];
  status?: ReactNode;
  title: ReactNode;
};

function createPlainTokens(code: string): RuntimeTokens {
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
            } satisfies RuntimeToken,
          ]
    ),
  };
}

function getSupportedLanguage(language?: string): BundledLanguage | null {
  if (!language) {
    return null;
  }

  return sharedCodePlugin.supportsLanguage(language as BundledLanguage)
    ? (language as BundledLanguage)
    : null;
}

function getDownloadExtension(language?: string) {
  if (!language) {
    return "txt";
  }

  return DOWNLOAD_EXTENSION_BY_LANGUAGE[language.toLowerCase()] ?? "txt";
}

function getDownloadFilename(section: RuntimeCodeSection) {
  const extension = getDownloadExtension(section.language);
  const baseName = section.downloadFilename ?? section.id;

  return baseName.includes(".") ? baseName : `${baseName}.${extension}`;
}

function renderTokenStyles(token: RuntimeToken): CSSProperties {
  return {
    backgroundColor: "transparent",
    color: token.color,
    ...token.htmlStyle,
  };
}

function useRuntimeTokens(code: string, language?: string) {
  const supportedLanguage = getSupportedLanguage(language);
  const rawTokens = useMemo(() => createPlainTokens(code), [code]);
  const [tokenized, setTokenized] = useState<RuntimeTokens>(rawTokens);
  const [isHighlighted, setIsHighlighted] = useState(false);

  useEffect(() => {
    if (!supportedLanguage) {
      setTokenized(rawTokens);
      setIsHighlighted(false);
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
            setIsHighlighted(true);
          }
        }
      ) ?? rawTokens;

    setTokenized(highlighted);
    setIsHighlighted(highlighted !== rawTokens);

    return () => {
      cancelled = true;
    };
  }, [code, rawTokens, supportedLanguage]);

  return { isHighlighted, tokenized };
}

function RuntimeCodeActions({ section }: { section: RuntimeCodeSection }) {
  const [isCopied, setIsCopied] = useState(false);
  const copiedTimerRef = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (copiedTimerRef.current !== null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    },
    []
  );

  const handleCopy = async () => {
    if (!navigator?.clipboard?.writeText) {
      return;
    }

    await navigator.clipboard.writeText(section.code);
    setIsCopied(true);

    if (copiedTimerRef.current !== null) {
      window.clearTimeout(copiedTimerRef.current);
    }

    copiedTimerRef.current = window.setTimeout(() => {
      setIsCopied(false);
      copiedTimerRef.current = null;
    }, 1_500);
  };

  const handleDownload = () => {
    const blob = new Blob([section.code], { type: "text/plain;charset=utf-8" });
    const objectUrl = URL.createObjectURL(blob);
    const anchor = document.createElement("a");

    anchor.href = objectUrl;
    anchor.download = getDownloadFilename(section);
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(objectUrl);
  };

  const CopyButtonIcon = isCopied ? CheckIcon : CopyIcon;

  return (
    <div
      className="pointer-events-none absolute right-2.5 top-1.5 z-10 flex opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-within:opacity-100"
      data-slot="runtime-code-actions"
    >
      <div className="pointer-events-auto flex items-start gap-1 border-0 bg-transparent p-0 shadow-none">
        <button
          aria-label="Copy code"
          className="flex size-3 items-center justify-center border-0 bg-transparent p-0 text-muted-foreground transition-colors hover:text-primary focus-visible:ring-2 focus-visible:ring-ring/30 outline-none"
          onClick={() => {
            void handleCopy();
          }}
          type="button"
        >
          <CopyButtonIcon size={12} />
        </button>
        <button
          aria-label="Download code"
          className="flex size-3 items-center justify-center border-0 bg-transparent p-0 text-muted-foreground transition-colors hover:text-primary focus-visible:ring-2 focus-visible:ring-ring/30 outline-none"
          onClick={handleDownload}
          type="button"
        >
          <DownloadIcon size={12} />
        </button>
      </div>
    </div>
  );
}

export function RuntimeCodeSurface({
  className,
  section,
  showLabel = true,
}: {
  className?: string;
  section: RuntimeCodeSection;
  showLabel?: boolean;
}) {
  const { isHighlighted, tokenized } = useRuntimeTokens(
    section.code,
    section.language
  );

  return (
    <div className="flex flex-col gap-0.5">
      {showLabel ? (
        <span
          className={cn(
            AI_RUNTIME_DENSITY.eyebrowText,
            "text-muted-foreground/80"
          )}
        >
          {section.label}
        </span>
      ) : null}
      <section
        className={cn(
          AI_CODE_SURFACE_CLASSNAME,
          section.tone === "error" && "border-destructive/30",
          className
        )}
        data-highlighted={isHighlighted ? "true" : "false"}
        data-slot="runtime-code-surface"
      >
        <RuntimeCodeActions section={section} />
        <div className="overflow-x-auto pl-2.5 pr-10 py-1.5">
          <pre
            className={cn(
              "m-0 whitespace-pre-wrap break-words font-mono text-[12px] leading-[14px]"
            )}
            style={{
              backgroundColor: "transparent",
              color: tokenized.fg,
            }}
          >
            <code>
              {tokenized.tokens.map((line, lineIndex) => (
                <span className="block" key={`runtime-line-${lineIndex}`}>
                  {line.length === 0
                    ? "\n"
                    : line.map((token, tokenIndex) => (
                        <span
                          key={`runtime-line-${lineIndex}-token-${tokenIndex}`}
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
      </section>
    </div>
  );
}

export function RuntimeCodePanel({
  className,
  icon,
  isRunning = false,
  sections,
  status,
  title,
  ...props
}: RuntimeCodePanelProps) {
  return (
    <StreamItem className={className} isRunning={isRunning} {...props}>
      <StreamItemTrigger icon={icon} label={title} status={status} />
      <StreamItemContent
        className={cn(
          "flex flex-col",
          AI_RUNTIME_DENSITY.sectionGap,
          AI_RUNTIME_DENSITY.bodyText
        )}
      >
        {sections.map((section) => (
          <RuntimeCodeSurface key={section.id} section={section} />
        ))}
      </StreamItemContent>
    </StreamItem>
  );
}

export type RuntimeCodePanelComponentProps = ComponentProps<typeof RuntimeCodePanel>;
