# Desktop Stream Item Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a shared running-state runtime shell for desktop AI stream items, then use it to render `Reasoning` and a fallback official tool item inside a dedicated dev playground page.

**Architecture:** Build a reusable `StreamItem` primitive under `desktop/components/ai` that owns run-aware open-state behavior and header shimmer treatment. Then compose semantic wrappers for `Reasoning` and `ToolCallItem`, and expose them in a `desktop/app/dev/stream` playground whose page stays server-rendered while the controls live in a leaf client component.

**Tech Stack:** Next.js App Router, React 19 client components, Tailwind utilities, Radix collapsible, `streamdown`, Vitest, Testing Library.

---

### Task 1: Add the shared `StreamItem` primitive with run-aware state

**Files:**
- Create: `desktop/components/ai/stream-item.tsx`
- Create: `desktop/components/ai/stream-item.test.tsx`

**Step 1: Write the failing test**

```tsx
import userEvent from "@testing-library/user-event";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
} from "@/components/ai/stream-item";

function Harness({
  isRunning,
  runKey,
}: {
  isRunning: boolean;
  runKey: number;
}) {
  return (
    <StreamItem
      autoCloseDelayMs={0}
      defaultOpen={false}
      defaultOpenWhenRunning
      isRunning={isRunning}
      runKey={runKey}
    >
      <StreamItemTrigger label="Reasoning" status="Thinking" />
      <StreamItemContent>stream body</StreamItemContent>
    </StreamItem>
  );
}

describe("StreamItem", () => {
  it("opens automatically when a run starts", () => {
    const { rerender } = render(<Harness isRunning={false} runKey={1} />);

    expect(screen.queryByText("stream body")).not.toBeInTheDocument();

    rerender(<Harness isRunning runKey={1} />);

    expect(screen.getByText("stream body")).toBeInTheDocument();
  });

  it("does not auto-reopen after a manual collapse in the same run", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness isRunning runKey={1} />);

    await user.click(screen.getByRole("button", { name: /reasoning/i }));
    expect(screen.queryByText("stream body")).not.toBeInTheDocument();

    rerender(<Harness isRunning runKey={1} />);

    expect(screen.queryByText("stream body")).not.toBeInTheDocument();
  });

  it("resets manual-collapse memory when the run key changes", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Harness isRunning runKey={1} />);

    await user.click(screen.getByRole("button", { name: /reasoning/i }));
    rerender(<Harness isRunning runKey={2} />);

    expect(screen.getByText("stream body")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test components/ai/stream-item.test.tsx`
Expected: FAIL because `@/components/ai/stream-item` does not exist yet.

**Step 3: Write minimal implementation**

```tsx
"use client";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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

type StreamItemContextValue = {
  isOpen: boolean;
  isRunning: boolean;
};

const StreamItemContext = createContext<StreamItemContextValue | null>(null);

export function StreamItem({
  children,
  isRunning = false,
  runKey,
  defaultOpen = false,
  defaultOpenWhenRunning = true,
  autoCloseDelayMs = 1000,
}: {
  children: React.ReactNode;
  isRunning?: boolean;
  runKey?: string | number;
  defaultOpen?: boolean;
  defaultOpenWhenRunning?: boolean;
  autoCloseDelayMs?: number;
}) {
  const [isOpen, setIsOpen] = useControllableState({
    defaultProp: defaultOpen,
  });
  const closedByUserRunKeyRef = useRef<string | number | undefined>(undefined);
  const autoOpenedRunKeyRef = useRef<string | number | undefined>(undefined);

  useEffect(() => {
    if (closedByUserRunKeyRef.current !== runKey) return;
    if (runKey !== undefined) return;
    closedByUserRunKeyRef.current = undefined;
  }, [runKey]);

  useEffect(() => {
    if (!isRunning || !defaultOpenWhenRunning) return;
    if (closedByUserRunKeyRef.current === runKey) return;
    setIsOpen(true);
    autoOpenedRunKeyRef.current = runKey;
  }, [defaultOpenWhenRunning, isRunning, runKey, setIsOpen]);

  useEffect(() => {
    if (isRunning) return;
    if (autoOpenedRunKeyRef.current !== runKey) return;

    const timer = window.setTimeout(() => {
      setIsOpen(false);
    }, autoCloseDelayMs);

    return () => window.clearTimeout(timer);
  }, [autoCloseDelayMs, isRunning, runKey, setIsOpen]);

  const value = useMemo(
    () => ({ isOpen, isRunning }),
    [isOpen, isRunning]
  );

  return <StreamItemContext.Provider value={value}>{children}</StreamItemContext.Provider>;
}

export const StreamItemTrigger = memo(function StreamItemTrigger({
  icon,
  label,
  status,
}: {
  icon?: React.ReactNode;
  label: string;
  status?: string;
}) {
  const context = useContext(StreamItemContext);
  if (!context) throw new Error("StreamItemTrigger must be inside StreamItem");

  return (
    <CollapsibleTrigger
      aria-label={label}
      className="flex w-full items-center gap-2 text-left text-sm text-muted-foreground"
      onClick={() => {
        if (context.isOpen) {
          // mark as user-closed for the current run
        }
      }}
    >
      {icon}
      <span>{label}</span>
      {status ? <span className="ml-auto text-xs">{status}</span> : null}
      <ChevronDownIcon className={cn("size-4 transition-transform", context.isOpen && "rotate-180")} />
    </CollapsibleTrigger>
  );
});

export function StreamItemContent({ children }: { children: React.ReactNode }) {
  return <CollapsibleContent className="pl-6 pt-2 text-sm text-muted-foreground">{children}</CollapsibleContent>;
}
```

Implementation notes:

- keep the actual item chrome borderless and background-free
- use context to share `isRunning` and `isOpen`
- track manual collapse per `runKey`
- ensure only runtime-opened items auto-close
- add a small shimmer helper directly in this file or as a tiny local helper if needed

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop test components/ai/stream-item.test.tsx`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/stream-item.tsx desktop/components/ai/stream-item.test.tsx
git commit -m "feat(desktop): add stream item runtime primitive"
```

### Task 2: Add `Reasoning` and `ToolCallItem` wrappers on top of the shared primitive

**Files:**
- Create: `desktop/components/ai/reasoning.tsx`
- Create: `desktop/components/ai/tool-call-item.tsx`
- Create: `desktop/components/ai/index.ts`
- Create: `desktop/components/ai/reasoning.test.tsx`
- Create: `desktop/components/ai/tool-call-item.test.tsx`

**Step 1: Write the failing tests**

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Reasoning } from "@/components/ai/reasoning";
import { ToolCallItem } from "@/components/ai/tool-call-item";

describe("Reasoning", () => {
  it("renders streamed markdown content through the shared runtime shell", () => {
    render(
      <Reasoning isRunning runKey={1}>
        {"First line\n\n- item"}
      </Reasoning>
    );

    expect(screen.getByRole("button", { name: /reasoning/i })).toBeInTheDocument();
    expect(screen.getByText("First line")).toBeInTheDocument();
  });
});

describe("ToolCallItem", () => {
  it("renders fallback tool summaries for running and completed states", () => {
    const { rerender } = render(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning
        name="shell"
        runKey={1}
      />
    );

    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("cwd: /workspace")).toBeInTheDocument();

    rerender(
      <ToolCallItem
        inputSummary="cwd: /workspace"
        isRunning={false}
        name="shell"
        outputSummary="exit 0"
        runKey={1}
      />
    );

    expect(screen.getByText("Completed")).toBeInTheDocument();
    expect(screen.getByText("exit 0")).toBeInTheDocument();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm --dir desktop test components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx`
Expected: FAIL because the wrapper components do not exist yet.

**Step 3: Write minimal implementation**

```tsx
"use client";

import { BrainIcon, WrenchIcon } from "lucide-react";
import { Streamdown } from "streamdown";
import {
  StreamItem,
  StreamItemContent,
  StreamItemTrigger,
} from "@/components/ai/stream-item";

export function Reasoning({
  children,
  isRunning = false,
  runKey,
}: {
  children: string;
  isRunning?: boolean;
  runKey?: string | number;
}) {
  return (
    <StreamItem
      defaultOpen={false}
      defaultOpenWhenRunning
      isRunning={isRunning}
      runKey={runKey}
    >
      <StreamItemTrigger icon={<BrainIcon className="size-4" />} label="Reasoning" status={isRunning ? "Thinking" : "Completed"} />
      <StreamItemContent>
        <Streamdown>{children}</Streamdown>
      </StreamItemContent>
    </StreamItem>
  );
}

export function ToolCallItem({
  name,
  isRunning = false,
  runKey,
  inputSummary,
  outputSummary,
  errorSummary,
}: {
  name: string;
  isRunning?: boolean;
  runKey?: string | number;
  inputSummary?: string;
  outputSummary?: string;
  errorSummary?: string;
}) {
  const status = errorSummary
    ? "Failed"
    : isRunning
      ? "Running"
      : "Completed";

  return (
    <StreamItem
      defaultOpen={false}
      defaultOpenWhenRunning
      isRunning={isRunning}
      runKey={runKey}
    >
      <StreamItemTrigger icon={<WrenchIcon className="size-4" />} label={name} status={status} />
      <StreamItemContent>
        {inputSummary ? <p>{inputSummary}</p> : null}
        {outputSummary ? <p>{outputSummary}</p> : null}
        {errorSummary ? <p>{errorSummary}</p> : null}
      </StreamItemContent>
    </StreamItem>
  );
}
```

Implementation notes:

- keep the wrappers thin; all run-aware behavior stays in `StreamItem`
- export the new surface from `desktop/components/ai/index.ts`
- do not import the old `desktop/components/ai-elements/reasoning.tsx`

**Step 4: Run tests to verify they pass**

Run: `pnpm --dir desktop test components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/reasoning.tsx desktop/components/ai/tool-call-item.tsx desktop/components/ai/index.ts desktop/components/ai/reasoning.test.tsx desktop/components/ai/tool-call-item.test.tsx
git commit -m "feat(desktop): add reasoning and tool stream wrappers"
```

### Task 3: Build the dev stream playground page with dashed sample wrappers

**Files:**
- Create: `desktop/app/dev/stream/page.tsx`
- Create: `desktop/app/dev/stream/stream-playground.tsx`
- Create: `desktop/app/dev/stream/page.test.tsx`

**Step 1: Write the failing test**

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import StreamPage from "./page";

describe("StreamPage", () => {
  it("renders the stream playground samples", () => {
    render(<StreamPage />);

    expect(screen.getByText("Reasoning")).toBeInTheDocument();
    expect(screen.getByText("shell")).toBeInTheDocument();
    expect(screen.getByText("Manual Collapse")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test app/dev/stream/page.test.tsx`
Expected: FAIL because the route does not exist yet.

**Step 3: Write minimal implementation**

```tsx
// page.tsx
import { StreamPlayground } from "./stream-playground";

export default function StreamPage() {
  return <StreamPlayground />;
}

// stream-playground.tsx
"use client";

import { useState } from "react";
import { Example, ExampleWrapper } from "@/components/example";
import { Button } from "@/components/ui/button";
import { Reasoning, ToolCallItem } from "@/components/ai";

export function StreamPlayground() {
  const [runKey, setRunKey] = useState(1);
  const [isRunning, setIsRunning] = useState(true);
  const [content, setContent] = useState("Let me think through this.");

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 p-6">
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => setIsRunning(true)}>Start Run</Button>
        <Button onClick={() => setIsRunning(false)} variant="outline">Finish Run</Button>
        <Button onClick={() => setRunKey((value) => value + 1)} variant="outline">Next Run</Button>
        <Button onClick={() => setContent((value) => `${value} More tokens.`)} variant="outline">Inject Token</Button>
      </div>

      <ExampleWrapper className="md:grid-cols-2">
        <Example title="Reasoning" className="rounded-none border border-dashed">
          <Reasoning isRunning={isRunning} runKey={runKey}>
            {content}
          </Reasoning>
        </Example>
        <Example title="Running Tool" className="rounded-none border border-dashed">
          <ToolCallItem isRunning name="shell" runKey={runKey} inputSummary="cwd: /workspace" />
        </Example>
        <Example title="Completed Tool" className="rounded-none border border-dashed">
          <ToolCallItem isRunning={false} name="read" runKey={runKey} outputSummary="Cargo.toml loaded" />
        </Example>
        <Example title="Manual Collapse" className="rounded-none border border-dashed">
          <Reasoning isRunning={isRunning} runKey={runKey}>
            {"Collapse me during a run, then start the next run to reset the memory."}
          </Reasoning>
        </Example>
      </ExampleWrapper>
    </div>
  );
}
```

Implementation notes:

- keep `page.tsx` as a server component
- put stateful controls only in `stream-playground.tsx`
- use dashed sample wrappers only in the playground layer

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop test app/dev/stream/page.test.tsx`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/app/dev/stream/page.tsx desktop/app/dev/stream/stream-playground.tsx desktop/app/dev/stream/page.test.tsx
git commit -m "feat(desktop): add stream playground page"
```

### Task 4: Verify the whole desktop stream surface end-to-end

**Files:**
- Verify: `desktop/components/ai/stream-item.tsx`
- Verify: `desktop/components/ai/reasoning.tsx`
- Verify: `desktop/components/ai/tool-call-item.tsx`
- Verify: `desktop/app/dev/stream/page.tsx`
- Verify: `desktop/app/dev/stream/stream-playground.tsx`

**Step 1: Run the targeted desktop tests**

Run: `pnpm --dir desktop test components/ai/stream-item.test.tsx components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx app/dev/stream/page.test.tsx`
Expected: PASS.

**Step 2: Run the full desktop test suite**

Run: `pnpm --dir desktop test`
Expected: PASS.

**Step 3: Run desktop lint**

Run: `pnpm --dir desktop lint`
Expected: PASS.

**Step 4: Manual verification**

Run: `pnpm --dir desktop dev`

Check:

- `/dev/stream` loads without hydration issues
- running samples shimmer only on `icon + label`
- tool and reasoning items share the same trigger rhythm
- manual collapse does not auto-reopen during the same run
- `Next Run` resets the behavior
- dashed wrappers exist only around the demo samples

**Step 5: Commit**

```bash
git add desktop/components/ai desktop/app/dev/stream
git commit -m "test(desktop): verify stream item runtime surface"
```
