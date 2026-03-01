# LLM Annotation Frontend Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the frontend annotation workflow for mixed form + Quill content with single-item right panel editing, autosave draft, rule-driven dynamic fields, and anti-drift anchoring.

**Architecture:** Add a dedicated `/annotation` feature in `desktop` with a schema-driven left review pane and a fixed right annotation panel. Store all annotation interaction in a dedicated Zustand store, use pure utility modules for location identity and drift-guard logic, and integrate Quill selection through a delayed trigger controller. Keep rule catalog source as remote-first with local fallback.

**Tech Stack:** Next.js 16, React 19, TypeScript, Zustand, Quill, quill-delta, Vitest, Testing Library

---

**Execution Skills:** `@test-driven-development`, `@systematic-debugging`, `@verification-before-completion`, `@requesting-code-review`

**Workspace:** `/Users/wanyaozhong/Projects/argusx`

### Task 1: Establish Desktop Test Harness

**Files:**
- Modify: `desktop/package.json`
- Create: `desktop/vitest.config.ts`
- Create: `desktop/test/setup.ts`
- Create: `desktop/lib/utils.test.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/utils.test.ts
import { describe, expect, it } from "vitest";
import { cn } from "@/lib/utils";

describe("cn", () => {
  it("keeps the latest Tailwind utility", () => {
    expect(cn("px-2", "px-4")).toBe("px-4");
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm exec vitest run lib/utils.test.ts
```
Expected: FAIL with missing `vitest` command or config.

**Step 3: Write minimal implementation**

- Add scripts in `desktop/package.json`:
  - `"test": "vitest run"`
  - `"test:watch": "vitest"`
- Install deps:
```bash
cd desktop && pnpm add -D vitest jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event
```
- Add config:
```ts
// desktop/vitest.config.ts
import { defineConfig } from "vitest/config";
import path from "node:path";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["./test/setup.ts"],
    globals: true,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
    },
  },
});
```

```ts
// desktop/test/setup.ts
import "@testing-library/jest-dom";
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/utils.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/package.json desktop/pnpm-lock.yaml desktop/vitest.config.ts desktop/test/setup.ts desktop/lib/utils.test.ts
git commit -m "test(desktop): add vitest harness"
```

### Task 2: Implement Location Model and Identity Helpers

**Files:**
- Create: `desktop/lib/annotation/types.ts`
- Create: `desktop/lib/annotation/location.ts`
- Create: `desktop/lib/annotation/location.test.ts`
- Create: `desktop/lib/annotation/index.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/annotation/location.test.ts
import { describe, expect, it } from "vitest";
import { createLocationFingerprint, isRichTextLocation } from "@/lib/annotation/location";
import type { AnnotationLocation } from "@/lib/annotation/types";

const rich: AnnotationLocation = {
  source_type: "rich_text_selection",
  panel: "paragraph_detail",
  section_id: "sec-1",
  field_key: "paragraph.summary",
  node_id: "node-22",
  start_offset: 12,
  end_offset: 19,
  selected_text: "违规片段",
};

describe("location helpers", () => {
  it("creates stable fingerprints", () => {
    expect(createLocationFingerprint(rich)).toBe(
      "paragraph_detail|sec-1|paragraph.summary|node-22|12|19",
    );
  });

  it("detects rich text location", () => {
    expect(isRichTextLocation(rich)).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/location.test.ts
```
Expected: FAIL with module not found.

**Step 3: Write minimal implementation**

```ts
// desktop/lib/annotation/types.ts
export type SourceType = "plain_field" | "rich_text_selection";

export type AnnotationLocation = {
  source_type: SourceType;
  panel: "basic_info" | "paragraph_detail";
  section_id: string;
  field_key: string;
  node_id: string;
  start_offset: number | null;
  end_offset: number | null;
  selected_text: string;
};
```

```ts
// desktop/lib/annotation/location.ts
import type { AnnotationLocation } from "./types";

export function createLocationFingerprint(location: AnnotationLocation): string {
  return [
    location.panel,
    location.section_id,
    location.field_key,
    location.node_id,
    location.start_offset ?? "null",
    location.end_offset ?? "null",
  ].join("|");
}

export function isRichTextLocation(location: AnnotationLocation): boolean {
  return location.source_type === "rich_text_selection";
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/location.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/annotation
git commit -m "feat(annotation): add location model and fingerprint helpers"
```

### Task 3: Implement Rule Catalog Resolver (Remote First + Local Fallback)

**Files:**
- Create: `desktop/lib/annotation/rules-fallback.ts`
- Create: `desktop/lib/annotation/rule-catalog.ts`
- Create: `desktop/lib/annotation/rule-catalog.test.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/annotation/rule-catalog.test.ts
import { describe, expect, it } from "vitest";
import { resolveRuleCatalog } from "@/lib/annotation/rule-catalog";
import { fallbackRules } from "@/lib/annotation/rules-fallback";

describe("resolveRuleCatalog", () => {
  it("returns remote rules on success", async () => {
    const remote = [{ code: "R1", label: "违规1", description: "d", version: 1, schema: [] }];
    const data = await resolveRuleCatalog(async () => remote, fallbackRules);
    expect(data.source).toBe("remote");
    expect(data.items).toEqual(remote);
  });

  it("falls back when remote throws", async () => {
    const data = await resolveRuleCatalog(async () => {
      throw new Error("network");
    }, fallbackRules);
    expect(data.source).toBe("fallback");
    expect(data.items).toEqual(fallbackRules);
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/rule-catalog.test.ts
```
Expected: FAIL with missing module exports.

**Step 3: Write minimal implementation**

- Define `RuleCatalogItem` and `RuleFieldSchema` in `types.ts`.
- Implement resolver:

```ts
// desktop/lib/annotation/rule-catalog.ts
import type { RuleCatalogItem } from "./types";

export async function resolveRuleCatalog(
  fetchRemote: () => Promise<RuleCatalogItem[]>,
  fallback: RuleCatalogItem[],
): Promise<{ source: "remote" | "fallback"; items: RuleCatalogItem[] }> {
  try {
    const remote = await fetchRemote();
    if (remote.length > 0) {
      return { source: "remote", items: remote };
    }
    return { source: "fallback", items: fallback };
  } catch {
    return { source: "fallback", items: fallback };
  }
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/rule-catalog.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/annotation/types.ts desktop/lib/annotation/rules-fallback.ts desktop/lib/annotation/rule-catalog.ts desktop/lib/annotation/rule-catalog.test.ts
git commit -m "feat(annotation): add rule catalog resolver with fallback"
```

### Task 4: Implement Annotation Reducer + Zustand Store

**Files:**
- Create: `desktop/lib/annotation/state.ts`
- Create: `desktop/lib/annotation/reducer.ts`
- Create: `desktop/lib/annotation/reducer.test.ts`
- Create: `desktop/lib/stores/annotation-store.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/annotation/reducer.test.ts
import { describe, expect, it } from "vitest";
import { annotationReducer, initialAnnotationState } from "@/lib/annotation/reducer";

const location = {
  source_type: "plain_field" as const,
  panel: "basic_info" as const,
  section_id: "base",
  field_key: "case_title",
  node_id: "case_title",
  start_offset: null,
  end_offset: null,
  selected_text: "",
};

describe("annotationReducer", () => {
  it("reuses existing annotation at same location", () => {
    const first = annotationReducer(initialAnnotationState, { type: "OPEN", location });
    const second = annotationReducer(first, { type: "OPEN", location });
    expect(second.items.length).toBe(1);
    expect(second.activeId).toBe(first.items[0].id);
  });

  it("autosaves previous draft when switching location", () => {
    const first = annotationReducer(initialAnnotationState, { type: "OPEN", location });
    const next = annotationReducer(first, {
      type: "OPEN",
      location: { ...location, field_key: "case_summary", node_id: "case_summary" },
    });
    const prev = next.items.find((i) => i.location.field_key === "case_title");
    expect(prev?.status).toBe("draft");
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/reducer.test.ts
```
Expected: FAIL with reducer module missing.

**Step 3: Write minimal implementation**

- Add `AnnotationDraft`, `AnnotationState`, `AnnotationAction` types.
- Implement reducer transitions: `OPEN`, `UPDATE_RULE`, `UPDATE_PAYLOAD`, `SUBMIT_SUCCESS`, `MARK_ORPHANED`.
- Wrap reducer in Zustand store:

```ts
// desktop/lib/stores/annotation-store.ts
import { create } from "zustand";
import { annotationReducer, initialAnnotationState } from "@/lib/annotation/reducer";

export const useAnnotationStore = create<{
  state: typeof initialAnnotationState;
  dispatch: (action: import("@/lib/annotation/state").AnnotationAction) => void;
}>((set) => ({
  state: initialAnnotationState,
  dispatch: (action) => set((prev) => ({ state: annotationReducer(prev.state, action) })),
}));
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/reducer.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/annotation/state.ts desktop/lib/annotation/reducer.ts desktop/lib/annotation/reducer.test.ts desktop/lib/stores/annotation-store.ts
git commit -m "feat(annotation): add reducer-driven annotation store"
```

### Task 5: Implement Quill Delayed Selection Trigger

**Files:**
- Create: `desktop/lib/annotation/quill-selection-controller.ts`
- Create: `desktop/lib/annotation/quill-selection-controller.test.ts`
- Create: `desktop/hooks/use-quill-selection-anchor.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/annotation/quill-selection-controller.test.ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQuillSelectionController } from "@/lib/annotation/quill-selection-controller";

describe("quill selection controller", () => {
  beforeEach(() => vi.useFakeTimers());

  it("emits once after 300ms", () => {
    const onFire = vi.fn();
    const ctl = createQuillSelectionController({ delayMs: 300, onFire });

    ctl.onSelectionChange({ index: 5, length: 3 });
    vi.advanceTimersByTime(299);
    expect(onFire).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onFire).toHaveBeenCalledTimes(1);
  });

  it("cancels when selection collapses", () => {
    const onFire = vi.fn();
    const ctl = createQuillSelectionController({ delayMs: 300, onFire });

    ctl.onSelectionChange({ index: 5, length: 3 });
    ctl.onSelectionChange({ index: 5, length: 0 });
    vi.advanceTimersByTime(300);

    expect(onFire).not.toHaveBeenCalled();
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/quill-selection-controller.test.ts
```
Expected: FAIL with missing module.

**Step 3: Write minimal implementation**

```ts
// desktop/lib/annotation/quill-selection-controller.ts
export function createQuillSelectionController({
  delayMs,
  onFire,
}: {
  delayMs: number;
  onFire: (range: { index: number; length: number }) => void;
}) {
  let timer: ReturnType<typeof setTimeout> | null = null;

  return {
    onSelectionChange(range: { index: number; length: number } | null) {
      if (timer) clearTimeout(timer);
      if (!range || range.length <= 0) return;
      timer = setTimeout(() => onFire(range), delayMs);
    },
    dispose() {
      if (timer) clearTimeout(timer);
    },
  };
}
```

- Add React hook wrapper `useQuillSelectionAnchor` that wires controller lifecycle.

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/quill-selection-controller.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/annotation/quill-selection-controller.ts desktop/lib/annotation/quill-selection-controller.test.ts desktop/hooks/use-quill-selection-anchor.ts
git commit -m "feat(annotation): add delayed quill selection trigger"
```

### Task 6: Implement Drift Guard with Delta Position Transform

**Files:**
- Modify: `desktop/package.json`
- Create: `desktop/lib/annotation/drift-guard.ts`
- Create: `desktop/lib/annotation/drift-guard.test.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/annotation/drift-guard.test.ts
import Delta from "quill-delta";
import { describe, expect, it } from "vitest";
import { reanchorByDelta } from "@/lib/annotation/drift-guard";

describe("reanchorByDelta", () => {
  it("shifts offsets after insertion before range", () => {
    const delta = new Delta().retain(3).insert("XYZ");
    const out = reanchorByDelta({ start: 10, end: 15 }, delta);
    expect(out).toEqual({ start: 13, end: 18 });
  });

  it("marks orphaned when selected text mismatches", () => {
    const delta = new Delta().retain(1).insert("A");
    const out = reanchorByDelta(
      { start: 2, end: 5, selectedText: "foo", currentTextAtRange: "bar" },
      delta,
    );
    expect(out.status).toBe("orphaned");
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/drift-guard.test.ts
```
Expected: FAIL due to missing `quill-delta` or module.

**Step 3: Write minimal implementation**

- Install dependency:
```bash
cd desktop && pnpm add quill-delta
```

```ts
// desktop/lib/annotation/drift-guard.ts
import Delta from "quill-delta";

export function reanchorByDelta(
  input: {
    start: number;
    end: number;
    selectedText?: string;
    currentTextAtRange?: string;
  },
  delta: Delta,
): { start: number; end: number; status?: "ok" | "orphaned" } {
  const start = delta.transformPosition(input.start);
  const end = delta.transformPosition(input.end);

  if (
    input.selectedText !== undefined &&
    input.currentTextAtRange !== undefined &&
    input.selectedText !== input.currentTextAtRange
  ) {
    return { start, end, status: "orphaned" };
  }

  return { start, end, status: "ok" };
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/annotation/drift-guard.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/package.json desktop/pnpm-lock.yaml desktop/lib/annotation/drift-guard.ts desktop/lib/annotation/drift-guard.test.ts
git commit -m "feat(annotation): add drift guard delta transform"
```

### Task 7: Build Annotation Route and Workspace Skeleton

**Files:**
- Create: `desktop/app/annotation/page.tsx`
- Create: `desktop/components/features/annotation/annotation-page.tsx`
- Create: `desktop/components/features/annotation/annotation-workspace.tsx`
- Create: `desktop/components/features/annotation/mock-review-data.ts`
- Create: `desktop/components/features/annotation/annotation-workspace.test.tsx`
- Modify: `desktop/components/features/index.ts`

**Step 1: Write the failing test**

```tsx
// desktop/components/features/annotation/annotation-workspace.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AnnotationWorkspace } from "@/components/features/annotation/annotation-workspace";

describe("AnnotationWorkspace", () => {
  it("renders left and right regions", () => {
    render(<AnnotationWorkspace />);
    expect(screen.getByTestId("review-left-pane")).toBeInTheDocument();
    expect(screen.getByTestId("annotation-right-panel")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/annotation-workspace.test.tsx
```
Expected: FAIL with missing component.

**Step 3: Write minimal implementation**

- Create `/annotation` route and render feature page.
- Create workspace with two-column layout and placeholder content matching design structure.

```tsx
// desktop/app/annotation/page.tsx
import { AnnotationPage } from "@/components/features/annotation/annotation-page";

export default function Page() {
  return <AnnotationPage />;
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/annotation-workspace.test.tsx
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/app/annotation/page.tsx desktop/components/features/annotation desktop/components/features/index.ts
git commit -m "feat(annotation): add annotation workspace route skeleton"
```

### Task 8: Implement Right Annotation Panel with Dynamic Rule Fields

**Files:**
- Create: `desktop/components/features/annotation/right-annotation-panel.tsx`
- Create: `desktop/components/features/annotation/rule-dynamic-fields.tsx`
- Create: `desktop/components/features/annotation/right-annotation-panel.test.tsx`
- Modify: `desktop/components/features/annotation/annotation-workspace.tsx`

**Step 1: Write the failing test**

```tsx
// desktop/components/features/annotation/right-annotation-panel.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { RightAnnotationPanel } from "@/components/features/annotation/right-annotation-panel";

describe("RightAnnotationPanel", () => {
  it("reveals dynamic fields after rule selection", async () => {
    const user = userEvent.setup();
    render(<RightAnnotationPanel />);

    await user.click(screen.getByRole("combobox", { name: "违规检查项" }));
    await user.click(screen.getByRole("option", { name: "事实一致性" }));

    expect(screen.getByLabelText("问题说明")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/right-annotation-panel.test.tsx
```
Expected: FAIL with missing fields.

**Step 3: Write minimal implementation**

- Render read-only location fields.
- Render rule select using catalog from store/provider.
- Render dynamic schema fields based on selected rule.
- Keep submit disabled until required fields are valid.

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/right-annotation-panel.test.tsx
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/right-annotation-panel.tsx desktop/components/features/annotation/rule-dynamic-fields.tsx desktop/components/features/annotation/right-annotation-panel.test.tsx desktop/components/features/annotation/annotation-workspace.tsx
git commit -m "feat(annotation): add right panel dynamic rule form"
```

### Task 9: Wire Left Pane Triggers and Quill Fields to Store

**Files:**
- Create: `desktop/components/features/annotation/left-review-pane.tsx`
- Create: `desktop/components/features/annotation/basic-info-form.tsx`
- Create: `desktop/components/features/annotation/paragraph-panel.tsx`
- Create: `desktop/components/features/annotation/quill-review-field.tsx`
- Create: `desktop/components/features/annotation/left-review-pane.test.tsx`
- Modify: `desktop/components/features/annotation/annotation-workspace.tsx`

**Step 1: Write the failing test**

```tsx
// desktop/components/features/annotation/left-review-pane.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { AnnotationWorkspace } from "@/components/features/annotation/annotation-workspace";

describe("left pane trigger", () => {
  it("opens right panel when plain field is clicked", async () => {
    const user = userEvent.setup();
    render(<AnnotationWorkspace />);

    await user.click(screen.getByTestId("annotatable-field-case_title"));

    expect(screen.getByDisplayValue("case_title")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/left-review-pane.test.tsx
```
Expected: FAIL with no interaction wiring.

**Step 3: Write minimal implementation**

- Build top/bottom collapsible structure for left pane.
- Use tree nav + content split for paragraph section.
- Add 4 `QuillReviewField` instances.
- On plain field click: dispatch `OPEN` with plain location.
- On quill range stop (300ms): dispatch `OPEN` with rich location.
- Apply highlight class to annotated targets.

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- components/features/annotation/left-review-pane.test.tsx
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/left-review-pane.tsx desktop/components/features/annotation/basic-info-form.tsx desktop/components/features/annotation/paragraph-panel.tsx desktop/components/features/annotation/quill-review-field.tsx desktop/components/features/annotation/left-review-pane.test.tsx desktop/components/features/annotation/annotation-workspace.tsx
git commit -m "feat(annotation): wire left pane annotation triggers"
```

### Task 10: Add API Adapters and Integrate Autosave/Submit Flows

**Files:**
- Create: `desktop/lib/api/annotation.ts`
- Create: `desktop/lib/annotation/loaders.ts`
- Create: `desktop/lib/api/annotation.test.ts`
- Modify: `desktop/components/features/annotation/annotation-page.tsx`
- Modify: `desktop/lib/stores/annotation-store.ts`

**Step 1: Write the failing test**

```ts
// desktop/lib/api/annotation.test.ts
import { describe, expect, it, vi } from "vitest";
import { loadRuleCatalog } from "@/lib/annotation/loaders";

describe("loadRuleCatalog", () => {
  it("returns fallback data when remote fails", async () => {
    const remote = vi.fn().mockRejectedValue(new Error("network"));
    const out = await loadRuleCatalog(remote);
    expect(out.source).toBe("fallback");
    expect(out.items.length).toBeGreaterThan(0);
  });
});
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd desktop && pnpm test -- lib/api/annotation.test.ts
```
Expected: FAIL with missing loaders/api modules.

**Step 3: Write minimal implementation**

- Implement API layer methods:
  - `fetchRuleCatalog`
  - `fetchAnnotations(docId)`
  - `upsertAnnotationDraft(payload)`
  - `submitAnnotation(id)`
- In page init, load remote rules then fallback.
- Wire store side effects:
  - on field edits: throttled autosave draft
  - on submit: call submit endpoint and update status

**Step 4: Run test to verify it passes**

Run:
```bash
cd desktop && pnpm test -- lib/api/annotation.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/api/annotation.ts desktop/lib/annotation/loaders.ts desktop/lib/api/annotation.test.ts desktop/components/features/annotation/annotation-page.tsx desktop/lib/stores/annotation-store.ts
git commit -m "feat(annotation): integrate remote-first catalog and autosave submit"
```

### Task 11: Verification, QA Checklist, and Final Integration Commit

**Files:**
- Create: `docs/plans/2026-03-01-annotation-frontend-qa-checklist.md`
- Modify: `desktop/app/page.tsx` (optional quick link to `/annotation`)

**Step 1: Write failing integration expectation (manual checklist first)**

- Draft manual checklist items for:
  - plain field annotation
  - quill 300ms delayed trigger
  - autosave on switch
  - duplicate location enters edit mode
  - orphaned state visibility

**Step 2: Run full verification before claiming done**

Run:
```bash
cd desktop && pnpm lint
cd desktop && pnpm test
cd desktop && pnpm build
```
Expected: all PASS.

**Step 3: Fix any failures minimally (debug with `@systematic-debugging`)**

- Apply smallest safe fix per failing area.

**Step 4: Re-run verification**

Run:
```bash
cd desktop && pnpm lint && pnpm test && pnpm build
```
Expected: all PASS.

**Step 5: Commit**

```bash
git add docs/plans/2026-03-01-annotation-frontend-qa-checklist.md desktop/app/page.tsx
git commit -m "chore(annotation): finalize verification and qa checklist"
```

