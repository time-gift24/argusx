# Desktop Stream Item Runtime Design

Date: 2026-03-06
Status: Approved
Scope: Desktop AI stream item runtime surface and dev playground only

## 1. Goal

Define one shared runtime surface for in-progress AI turn items in the desktop app so `Reasoning` and tool items behave consistently while streaming.

This phase only covers the foundational running-state capability:

- compact, borderless item styling
- shared open/close behavior while a run is active
- shared shimmer treatment for `icon + label`
- one fallback rendering path for official tool calls
- a dedicated dev page for visual validation

It does not yet include production chat integration or per-tool custom visual treatments.

## 2. Context

Current desktop state:

- [`desktop/components/ai-elements/reasoning.tsx`](/Users/wanyaozhong/Projects/argusx/desktop/components/ai-elements/reasoning.tsx) already contains local auto-open and auto-close logic tied to `isStreaming`.
- The `ai-elements` directory reflects vendored or reference-oriented primitives and should not become the home for ArgusX-specific abstractions.
- The user wants a unified runtime capability first, not final visual designs for each individual tool type.
- Tool runtime design in [`docs/plans/2026-03-06-tool-runtime-redesign-design.md`](/Users/wanyaozhong/Projects/argusx/docs/plans/2026-03-06-tool-runtime-redesign-design.md) confirms that builtin and MCP-backed tools share one conceptual runtime layer, so the desktop UI should also start from one shared surface.

The desktop app currently has no focused stream playground, so validating this behavior inside a dedicated dev route is the safest first step.

## 3. Approved Decisions

1. New runtime-surface components live under `desktop/components/ai`, not `desktop/components/ai-elements`.
2. `ai-elements` remains reference material only; new desktop abstractions must not deepen that dependency.
3. Use a shared base primitive approach, not ad hoc duplicated logic in `Reasoning` and each tool item.
4. Introduce a `runKey` concept so a new run resets item behavior cleanly.
5. Manual collapse wins within the current run. Streaming updates must not auto-reopen an item the user collapsed during that run.
6. Running-state shimmer applies only to header emphasis, specifically `icon + label`, not the whole content block.
7. Tool items use one unified fallback visual treatment in this phase; specific tool skins come later.
8. The dev page uses dashed outer wrappers around each sample card for demonstration only. The actual runtime item remains borderless.

## 4. Options Considered

### Option 1: Hook-Only Reuse

Extract a `useStreamItemState` hook and let each item manage its own structure and styling.

Pros:

- small migration from the existing `Reasoning`
- minimal initial file churn

Cons:

- visual drift remains likely
- harder to guarantee uniform trigger structure and shimmer treatment
- future tool item variants would still tend to fork

### Option 2: Shared Base Primitive

Create a base `StreamItem` primitive with shared runtime behavior and header/content structure, then build `Reasoning` and `ToolCallItem` on top.

Pros:

- one source of truth for running-state behavior
- consistent visual rhythm across reasoning and tool items
- easy to add later tool-specific content renderers without re-implementing the runtime shell

Cons:

- requires an explicit abstraction layer up front

### Option 3: Dev Page First, Components Later

Build a hard-coded playground page first and extract shared primitives only after visual validation.

Pros:

- fastest route to pixels

Cons:

- high rewrite risk
- pushes core runtime behavior decisions into throwaway code

### Selected Approach

Option 2 is approved.

## 5. Target Architecture

### 5.1 File Layout

```text
desktop/
├── app/
│   └── dev/
│       └── stream/
│           ├── page.tsx
│           └── stream-playground.tsx
└── components/
    └── ai/
        ├── index.ts
        ├── stream-item.tsx
        ├── reasoning.tsx
        ├── tool-call-item.tsx
        ├── stream-item.test.tsx
        ├── reasoning.test.tsx
        └── tool-call-item.test.tsx
```

Notes:

- `page.tsx` should stay a server component.
- interactive demo controls belong in `stream-playground.tsx` as a client component.
- the new `Reasoning` implementation should not depend on `desktop/components/ai-elements/reasoning.tsx`.

### 5.2 Base Primitive

The shared primitive should provide:

- `StreamItem`
- `StreamItemTrigger`
- `StreamItemContent`
- a small running-label helper for shimmer treatment

The primitive owns:

- current open state
- per-run memory of whether the user manually collapsed the item
- optional finish duration summary
- delayed auto-close after a run ends

### 5.3 Semantic Wrappers

Two wrappers should exist in this phase:

- `Reasoning`
  - renders streamed markdown content
  - uses the shared runtime shell
- `ToolCallItem`
  - renders official tool fallback content
  - uses the same shell with a different header label and summary body

## 6. Runtime State Model

The base primitive should accept these core inputs:

- `isRunning: boolean`
- `runKey?: string | number`
- `defaultOpen?: boolean`
- `defaultOpenWhenRunning?: boolean`
- `autoCloseOnFinish?: boolean`
- `autoCloseDelayMs?: number`

### 6.1 Behavior Rules

1. Initial render uses `defaultOpen` for the static resting state.
2. When `isRunning` becomes `true`, the item auto-opens if:
   - `defaultOpenWhenRunning` is enabled, and
   - the user has not manually collapsed the item during the current `runKey`.
3. While `isRunning` is `true`, `icon + label` shimmer continuously.
4. If the user manually collapses an item during the current run, it stays collapsed even if more tokens arrive.
5. When `isRunning` becomes `false`, the item may auto-close after `autoCloseDelayMs`, but only if the open state was runtime-driven rather than user-driven.
6. When `runKey` changes, per-run collapse memory resets and the item can auto-open again for the next run.

### 6.2 Rationale

`isRunning` alone is not enough because it cannot distinguish:

- a resumed stream in the same run
- a truly new run that should reset auto-open behavior

`runKey` solves that without coupling the primitive to backend event details.

## 7. Visual Design Rules

### 7.1 Runtime Item

Runtime items must be:

- borderless
- shadowless
- background-free by default
- compact in vertical spacing
- single-line in the trigger whenever content length allows

Header structure:

- leading icon
- title or label
- compact status text
- trailing chevron

Allowed emphasis:

- `icon + label` shimmer while running
- muted status text such as `Thinking`, `Running`, `Completed`, or `Failed`
- subtle content indentation for hierarchy

Disallowed in this phase:

- card chrome
- per-item outline borders
- full-content shimmer
- large status pills or badges
- tool-specific ornamental styling

### 7.2 Dev Playground Layout

The dev page should present examples in clearly isolated sample wrappers:

- each sample gets a dashed outer border
- the dashed wrapper belongs to the playground only, not the core component
- sample wrappers should have enough padding to inspect compact spacing without crowding
- mobile layout is one column
- desktop layout is two columns

The page should include:

- one reasoning sample that streams tokens
- one running tool fallback sample
- one finished tool fallback sample
- one sample demonstrating manual collapse within a run
- one control strip for starting a run, finishing a run, incrementing `runKey`, and injecting more content

## 8. Tool Fallback Rendering

This phase defines one fallback renderer for official tools.

Header:

- generic tool icon
- tool display name
- compact status text

Content:

- input summary if available
- output summary if available
- error summary if the tool failed

The fallback renderer is intentionally plain. Future per-tool renderers may replace only the content region while preserving the same shared header and runtime behavior.

## 9. Testing Strategy

The implementation must be test-first and validate both behavior and rendering shape.

Required test coverage:

- auto-open when a run starts
- no auto-reopen after manual collapse within the same `runKey`
- auto-close after finish only when runtime-opened
- reset of manual-collapse memory when `runKey` changes
- reasoning wrapper still renders streamed markdown content
- tool fallback renders running and completed summaries correctly
- dev page exposes the expected demo sections

## 10. Non-Goals

This phase does not include:

- wiring the new components into the production chat route
- live runtime integration with provider or turn events
- per-tool bespoke layouts
- message list virtualization
- final typography or color-system overhaul

## 11. Success Criteria

This work is successful when:

1. `Reasoning` and tool fallback items share one runtime shell and behave identically during runs.
2. The new components live entirely under `desktop/components/ai`.
3. The dev playground makes the behavior obvious without using production chat data.
4. The actual runtime item surface remains clean, compact, and borderless.
5. All new behavior is covered by targeted desktop tests.
