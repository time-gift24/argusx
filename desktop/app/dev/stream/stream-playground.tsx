"use client";

import { useEffect, useState } from "react";

import { Reasoning, ToolCallItem } from "@/components/ai";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const INITIAL_REASONING =
  "Let me map the request first.\n\n- unify the running surface\n- keep the shell compact\n- let tools reuse the same behavior";

const MANUAL_COLLAPSE_HINT =
  "Collapse this item while it is running, then keep injecting tokens. It should stay closed until the next run starts.";

const STREAM_TICK_MS = 700;
const STREAMED_TOKENS = [
  "streamed token arrived · runtime-open state comes from one shared shell",
  "streamed token arrived · manual collapse survives follow-up tokens in the same run",
  "streamed token arrived · next run resets collapse memory without reviving old user intent",
];

function SampleFrame({
  children,
  className,
  description,
  title,
}: {
  children: React.ReactNode;
  className?: string;
  description: string;
  title: string;
}) {
  return (
    <section
      className={cn(
        "flex min-w-0 flex-col gap-4 rounded-xl border border-dashed border-border/80 bg-background/70 p-4 sm:p-5",
        className
      )}
    >
      <div className="flex flex-col gap-1">
        <h2 className="text-sm font-medium text-foreground">{title}</h2>
        <p className="text-xs leading-5 text-muted-foreground">{description}</p>
      </div>
      <div className="min-w-0">{children}</div>
    </section>
  );
}

export function StreamPlayground() {
  const [runKey, setRunKey] = useState(1);
  const [isRunning, setIsRunning] = useState(true);
  const [reasoningText, setReasoningText] = useState(INITIAL_REASONING);
  const [manualCollapseText, setManualCollapseText] = useState(
    MANUAL_COLLAPSE_HINT
  );
  const [streamCursor, setStreamCursor] = useState(0);

  useEffect(() => {
    if (!isRunning || streamCursor >= STREAMED_TOKENS.length) {
      return;
    }

    const timer = window.setTimeout(() => {
      const token = `\n- ${STREAMED_TOKENS[streamCursor]}`;

      setReasoningText((value) => `${value}${token}`);
      setManualCollapseText((value) => `${value}${token}`);
      setStreamCursor((value) => value + 1);
    }, STREAM_TICK_MS);

    return () => window.clearTimeout(timer);
  }, [isRunning, streamCursor]);

  const resetStreamingSamples = () => {
    setReasoningText(INITIAL_REASONING);
    setManualCollapseText(MANUAL_COLLAPSE_HINT);
    setStreamCursor(0);
  };

  const handleNextRun = () => {
    setRunKey((value) => value + 1);
    setIsRunning(true);
    resetStreamingSamples();
  };

  const handleStartRun = () => {
    if (!isRunning) {
      resetStreamingSamples();
    }

    setIsRunning(true);
  };

  const handleInjectToken = () => {
    const token = "\n- streamed token arrived";

    setReasoningText((value) => `${value}${token}`);
    setManualCollapseText((value) => `${value}${token}`);
  };

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8 px-4 py-6 sm:px-6 lg:px-8">
      <header className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <span className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
            AI Runtime Surface
          </span>
          <div className="flex flex-col gap-2 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-2xl space-y-2">
              <h1 className="text-2xl font-semibold tracking-tight text-foreground">
                Stream component playground
              </h1>
              <p className="text-sm leading-6 text-muted-foreground">
                Validate the shared running-state shell before wiring it into
                the real chat stream. The item itself stays borderless; the
                dashed wrappers belong only to this demo page.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button onClick={handleStartRun} size="sm">
                Start Run
              </Button>
              <Button
                onClick={() => setIsRunning(false)}
                size="sm"
                variant="outline"
              >
                Finish Run
              </Button>
              <Button onClick={handleNextRun} size="sm" variant="outline">
                Next Run
              </Button>
              <Button onClick={handleInjectToken} size="sm" variant="outline">
                Inject Token
              </Button>
            </div>
          </div>
        </div>
        <div className="flex flex-wrap gap-x-6 gap-y-2 text-xs text-muted-foreground">
          <span>Run key: {runKey}</span>
          <span>Status: {isRunning ? "running" : "idle"}</span>
        </div>
      </header>

      <div className="grid gap-4 lg:grid-cols-2">
        <SampleFrame
          description="Shared reasoning shell with automatic open-on-run and compact markdown content."
          title="Reasoning"
        >
          <Reasoning isRunning={isRunning} runKey={runKey}>
            {reasoningText}
          </Reasoning>
        </SampleFrame>

        <SampleFrame
          description="Unified fallback rendering for official tools while they are still running."
          title="Running Tool"
        >
          <ToolCallItem
            inputSummary="pwd && ls"
            isRunning={isRunning}
            name="shell"
            outputSummary={isRunning ? undefined : "exit 0 · 148ms"}
            runKey={runKey}
          />
        </SampleFrame>

        <SampleFrame
          description="Finished tools keep the same shell but switch to their resting status."
          title="Completed Tool"
        >
          <ToolCallItem
            isRunning={false}
            name="read"
            outputSummary="Cargo.toml loaded · 2.1 KB"
            runKey={runKey}
          />
        </SampleFrame>

        <SampleFrame
          description="This sample is here only to verify that manual collapse wins until the next run resets the state."
          title="Manual Collapse"
        >
          <Reasoning isRunning={isRunning} runKey={runKey}>
            {manualCollapseText}
          </Reasoning>
        </SampleFrame>
      </div>
    </div>
  );
}
