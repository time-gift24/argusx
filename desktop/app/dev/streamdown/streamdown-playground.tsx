"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import { Streamdown } from "streamdown";

import {
  sharedStreamdownClassName,
  sharedStreamdownComponents,
  sharedStreamdownControls,
  sharedStreamdownIcons,
  sharedStreamdownPlugins,
  sharedStreamdownShikiTheme,
  sharedStreamdownTranslations,
} from "@/components/ai/streamdown";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const proseClassName = sharedStreamdownClassName;
const streamingTickMs = 28;

type SampleMarkdown = {
  id: string;
  title: string;
  content: string;
};

function stopInterval(intervalRef: { current: number | null }) {
  if (intervalRef.current !== null) {
    window.clearInterval(intervalRef.current);
    intervalRef.current = null;
  }
}

const SAMPLE_MARKDOWNS: SampleMarkdown[] = [
  {
    id: "basic",
    title: "Basic Markdown",
    content: [
      "# Heading 1",
      "",
      "This is a paragraph with **bold text** and *italic text*.",
      "",
      "## Heading 2",
      "",
      "Here's some `inline code`:",
      "",
      "```javascript",
      'const greeting = "Hello, world!";',
      "console.log(greeting);",
      "```",
      "",
      "- List item 1",
      "- List item 2",
      "- List item 3",
    ].join("\n"),
  },
  {
    id: "code",
    title: "Code Blocks",
    content: [
      "## Code Block Example",
      "",
      "```typescript",
      "interface User {",
      "  id: number;",
      "  name: string;",
      "  email: string;",
      "  role: \"admin\" | \"editor\" | \"viewer\";",
      "  tags: string[];",
      "  active: boolean;",
      "}",
      "",
      "type UserSummary = Pick<User, \"id\" | \"name\" | \"role\">;",
      "",
      "function normalizeTag(tag: string): string {",
      "  return tag.trim().toLowerCase();",
      "}",
      "",
      "function greet(user: User): string {",
      '  return "Hello, " + user.name + "!";',
      "}",
      "",
      "function toSummary(user: User): UserSummary {",
      "  return {",
      "    id: user.id,",
      "    name: user.name,",
      "    role: user.role,",
      "  };",
      "}",
      "",
      "const users: User[] = [",
      '  { id: 1, name: "Alice", email: "alice@example.com", role: "admin", tags: ["Core", " Ops "], active: true },',
      '  { id: 2, name: "Bob", email: "bob@example.com", role: "editor", tags: ["Docs", "Review"], active: true },',
      '  { id: 3, name: "Cara", email: "cara@example.com", role: "viewer", tags: ["beta"], active: false },',
      "];",
      "",
      "const activeUsers = users",
      "  .filter((user) => user.active)",
      "  .map((user) => ({",
      "    ...toSummary(user),",
      "    tags: user.tags.map(normalizeTag),",
      "  }));",
      "",
      "for (const user of activeUsers) {",
      "  console.log(greet(user));",
      "}",
      "```",
    ].join("\n"),
  },
  {
    id: "math",
    title: "Math Equations",
    content: [
      "## Math Equations",
      "",
      "Inline math: $E = mc^2$",
      "",
      "Block math:",
      "$$",
      "\\frac{a + b}{c^2 - ab} = \\lim_{x \\to \\infty} f(x)",
      "$$",
      "",
      "Another expression: $\\int_a^b f(x)\\,dx = \\sqrt{a^2 + b^2}$",
    ].join("\n"),
  },
  {
    id: "mermaid",
    title: "Mermaid Diagram",
    content: [
      "## Mermaid Diagram",
      "",
      "```mermaid",
      "graph TD",
      "  Client[Client] --> Browse[Browse Products]",
      "  Browse --> Cart[Cart]",
      "  Cart --> Checkout[Checkout]",
      "  Checkout --> Delivery[Delivery]",
      "```",
    ].join("\n"),
  },
  {
    id: "list",
    title: "Lists & Tasks",
    content: [
      "## Task List",
      "",
      "- [x] Complete documentation",
      "    - Review API surface",
      "    - Capture screenshots",
      "- [ ] Set up CI/CD pipeline",
      "    1. Add lint job",
      "    2. Add build step",
      "- [ ] Write unit tests",
      "- [ ] Review code changes",
      "- [ ] Deploy to production",
      "",
      "## Priority Queue",
      "",
      "1. Critical bug fix",
      "2. Feature implementation",
      "3. Code refactoring",
      "4. Documentation update",
      "",
      "Mixed-language sample: 支持中文、かな、한글 rendering.",
    ].join("\n"),
  },
];

function DemoCard({
  title,
  children,
  className,
}: {
  title: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "group relative overflow-hidden rounded-xl border border-border/50 bg-background/70 p-4 transition-colors duration-200 hover:border-border/80 hover:bg-background/90",
        className
      )}
    >
      <h3 className="mb-2.5 text-[12px] font-medium leading-[14px] text-foreground">
        {title}
      </h3>
      <div className="min-h-0">{children}</div>
    </section>
  );
}

function MarkdownPreview({
  content,
  isAnimating = false,
}: {
  content: string;
  isAnimating?: boolean;
}) {
  return (
    <Streamdown
      className={proseClassName}
      components={sharedStreamdownComponents}
      controls={sharedStreamdownControls}
      icons={sharedStreamdownIcons}
      isAnimating={isAnimating}
      plugins={sharedStreamdownPlugins}
      shikiTheme={sharedStreamdownShikiTheme}
      translations={sharedStreamdownTranslations}
    >
      {content}
    </Streamdown>
  );
}

function StreamingDemo() {
  const [currentDemoIndex, setCurrentDemoIndex] = useState(0);
  const [displayText, setDisplayText] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const intervalRef = useRef<number | null>(null);
  const cursorRef = useRef(0);

  const currentDemo = SAMPLE_MARKDOWNS[currentDemoIndex];
  const streamSource = currentDemo.content;

  useEffect(() => () => stopInterval(intervalRef), []);

  useEffect(() => {
    if (!isStreaming || isPaused) {
      stopInterval(intervalRef);
      return;
    }

    stopInterval(intervalRef);
    intervalRef.current = window.setInterval(() => {
      cursorRef.current += 1;
      const nextValue = streamSource.slice(0, cursorRef.current);

      setDisplayText(nextValue);

      if (cursorRef.current >= streamSource.length) {
        stopInterval(intervalRef);
        setIsStreaming(false);
        setIsPaused(false);
      }
    }, streamingTickMs);

    return () => stopInterval(intervalRef);
  }, [isPaused, isStreaming, streamSource]);

  const handleStartStream = () => {
    stopInterval(intervalRef);
    cursorRef.current = 0;
    setDisplayText("");
    setIsPaused(false);
    setIsStreaming(true);
  };

  const handlePauseStream = () => {
    stopInterval(intervalRef);
    setIsPaused(true);
  };

  const handleResumeStream = () => {
    cursorRef.current = displayText.length;
    setIsPaused(false);
    setIsStreaming(true);
  };

  const handleResetStream = () => {
    stopInterval(intervalRef);
    cursorRef.current = 0;
    setDisplayText("");
    setIsPaused(false);
    setIsStreaming(false);
  };

  const handleNextDemo = () => {
    handleResetStream();
    setCurrentDemoIndex((value) => (value + 1) % SAMPLE_MARKDOWNS.length);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2">
          <span className="text-[12px] font-medium leading-[14px] text-foreground">
            {currentDemo.title}
          </span>
          <span className="rounded-full bg-blue-500/10 px-2.5 py-1 text-[12px] font-medium leading-[14px] text-blue-600 dark:text-blue-300">
            Live
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={handleStartStream}
            size="sm"
            variant={isStreaming && !isPaused ? "outline" : "default"}
            disabled={isStreaming && !isPaused}
          >
            {isStreaming && !isPaused ? "Streaming..." : "Start"}
          </Button>
          {isStreaming ? (
            <Button
              onClick={isPaused ? handleResumeStream : handlePauseStream}
              size="sm"
              variant="outline"
            >
              {isPaused ? "Resume" : "Pause"}
            </Button>
          ) : null}
          <Button onClick={handleResetStream} size="sm" variant="outline">
            Reset
          </Button>
          <Button onClick={handleNextDemo} size="sm" variant="outline">
            Next Demo
          </Button>
        </div>
      </div>
      <div className="min-h-[400px] overflow-y-auto rounded-xl border border-border/50 bg-background p-4">
        {displayText ? (
          <MarkdownPreview content={displayText} isAnimating={isStreaming && !isPaused} />
        ) : (
          <div className="flex h-32 items-center justify-center text-[12px] leading-[14px] text-muted-foreground">
            Click &quot;Start&quot; to stream the current sample.
          </div>
        )}
      </div>
    </div>
  );
}

export function StreamdownPlayground() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-12 px-4 py-8 sm:px-6 lg:px-8">
      <header className="flex flex-col gap-6">
        <div className="flex flex-col gap-2">
          <span className="text-[12px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
            Markdown Rendering
          </span>
          <h1 className="text-4xl font-semibold tracking-tight text-foreground">
            Streamdown Playground
          </h1>
          <p className="max-w-2xl text-[12px] leading-[14px] text-muted-foreground">
            Real-time markdown rendering with code fences, math, mermaid diagrams, and CJK
            content. The page mirrors the existing dev playground style while keeping the
            sample data easy to maintain.
          </p>
        </div>
      </header>

      <div className="grid gap-6 lg:grid-cols-2">
        {SAMPLE_MARKDOWNS.map((sample) => (
          <DemoCard
            key={sample.id}
            className={sample.id === "basic" ? "lg:col-span-2" : undefined}
            title={sample.title}
          >
            <MarkdownPreview content={sample.content} />
          </DemoCard>
        ))}
      </div>

      <DemoCard title="Streaming Demo">
        <StreamingDemo />
      </DemoCard>
    </div>
  );
}
