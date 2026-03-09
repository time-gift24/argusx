# Desktop Task And Plan Display Design

## Goal

Refine the desktop chat runtime UI so that:

- `update_plan` output is shown as a floating plan card above the prompt input while a turn is running
- read-only builtin tool activity is grouped into a compact assistant-side task list instead of rendering as individual tool rows

This is a display-layer change for the desktop app. It does not change the
`Session -> Thread -> Turn` domain boundaries or the Rust turn event protocol.

## Current State

On current `main`:

- desktop chat stores the latest parsed `update_plan` payload on each turn as `latestPlan`
- `desktop/app/chat/page.tsx` renders `PlanQueue` inline inside each assistant turn
- `update_plan` tool rows are filtered out from the generic tool call list
- builtin read tools are exposed in desktop as `read`, `glob`, and `grep`
- read tool calls currently render through the generic `ToolCallItem` flow

This produces two UX problems:

1. execution plans become part of the chat transcript instead of acting like runtime guidance near the composer
2. read-heavy turns become noisy because each read call expands into a full tool row

## Scope

### In scope

- move `update_plan` rendering out of the assistant transcript area
- show the active plan only while the turn is running
- keep read-tool history attached to the assistant turn
- group `read`, `glob`, and `grep` tool calls into a compact AI Elements `Task` list
- default the read-tool task list to collapsed
- show only the latest 3 read tool calls per turn

### Out of scope

- changing Rust/Tauri event payloads
- changing turn persistence shape
- adding new builtin tools
- persisting task-list expanded/collapsed UI state
- making non-read tools use the `Task` UI

## Constraints

The following constraints must remain true:

1. `TurnDriver` remains the only single-turn execution engine.
2. Desktop UI state does not become persisted domain truth.
3. `update_plan` remains a tool result, but its desktop rendering becomes runtime-only.
4. Turn hydration continues to restore `latestPlan` and `toolCalls` without new fields.
5. Read tool grouping is a frontend display concern, not a new domain concept.

## Options Considered

### Option 1: UI-only reshaping on top of existing state

Keep current event and hydration flow. Derive the floating plan and read-tool task
group directly from the existing turn view model in React.

Pros:

- smallest change surface
- no protocol churn
- preserves current domain layering

Cons:

- frontend owns the read-tool whitelist

### Option 2: Introduce dedicated page-level runtime state for plan and read groups

Normalize `latestPlan` and grouped tool state into additional React state instead of
deriving them during render.

Pros:

- can make render paths more explicit

Cons:

- duplicates information already present in turn state
- increases state sync complexity without clear value

### Option 3: Add backend display metadata

Emit explicit display kinds from Tauri or Rust for floating plan vs read task items.

Pros:

- centralizes display classification

Cons:

- changes protocol for a purely desktop-presentational concern
- overfits the current builtin set

## Recommended Approach

Use Option 1.

Keep the current Rust/Tauri protocol unchanged and reshape the desktop UI in the
React layer only:

- derive one `activeFloatingPlan` from the latest running turn that has a `latestPlan`
- render `PlanQueue` inside a floating card above the prompt composer
- remove inline `PlanQueue` rendering from assistant turns
- derive a per-turn read-tool group from `read`, `glob`, and `grep`
- render that group with AI Elements `Task` primitives using a compact, collapsed summary

This keeps the change local to desktop presentation while preserving the domain
model described in `AGENTS.md`.

## UX Behavior

### Floating plan

- the plan card appears above the prompt input only when the current running turn has a parsed `latestPlan`
- the plan card is anchored in the existing composer shell area
- the plan card disappears as soon as the turn reaches `completed`, `failed`, or `cancelled`
- `update_plan` does not render as a tool row in the assistant transcript

### Read task summary

- only builtin `read`, `glob`, and `grep` are included
- the grouped section stays inside the assistant turn where the tool calls happened
- the section header text is `Summary`
- the section is collapsed by default
- the list is visually tighter than the generic tool row layout
- only the latest 3 read tool calls are shown
- history turns keep the grouped section after hydration

### Non-read tools

- all other tools continue to render through `ToolCallItem`
- no behavior change for permission UI, reasoning, markdown, or error display

## State And Data Flow

No new persisted fields are added.

The desktop page derives:

- `activeFloatingPlan`
  - source: the last turn where `status === "running"` and `latestPlan != null`
- `recentReadTasks`
  - source: each turn's `toolCalls`
  - filter: `name` in `{"read", "glob", "grep"}`
  - limit: last 3 items in existing order

Assistant turn rendering becomes:

1. assistant markdown
2. reasoning
3. read-task summary group
4. non-read, non-`update_plan` tool rows
5. turn error

Composer shell rendering becomes:

1. permission confirmation, if present
2. floating plan card, if `activeFloatingPlan` exists
3. prompt composer

The plan remains stored on the turn view model so event reduction and hydration stay
simple, but it is no longer rendered as transcript content.

## Component Design

### Existing components to keep

- `desktop/app/chat/page.tsx`
- `desktop/components/ai/plan-queue.tsx`
- `desktop/components/ai/tool-call-item.tsx`

### New desktop components

- `FloatingPlanCard`
  - wraps `PlanQueue` in the composer-shell floating container
  - owns spacing, border, and layering only
- `ReadTaskGroup`
  - uses AI Elements `Task` primitives installed via:
    - `npx ai-elements@latest add task`
  - accepts a narrow UI model instead of raw turn events

Suggested `ReadTaskGroup` input shape:

```ts
type ReadTaskItem = {
  callId: string;
  name: string;
  status: ToolCallView["status"];
  inputSummary?: string;
  outputSummary?: string;
  errorSummary?: string;
};
```

Page-level code remains responsible for classifying turn tool calls into:

- `update_plan`
- read tools
- everything else

## Error Handling

- invalid `plan-updated` payloads continue to be ignored
- no floating-plan placeholder renders when no valid plan exists
- if a read tool lacks summaries, the grouped item still renders with tool name and status
- if a turn has no read tools, the `Summary` group is omitted
- UI-only clipping to 3 items does not alter stored tool-call history

## Testing Strategy

### Page tests

- verify `PlanQueue` no longer renders inline in the assistant section
- verify a running turn with `latestPlan` renders a floating plan card above the composer
- verify the floating plan disappears after `turn-finished`
- verify read tools render in a `Summary` task group
- verify only the latest 3 read tool calls are shown
- verify non-read tools still render through `ToolCallItem`

### Component tests

- `ReadTaskGroup` is collapsed by default
- `Summary` toggle expands and collapses
- compact success and error summaries render correctly
- `FloatingPlanCard` renders the supplied plan without transcript coupling

### Regression tests

- hydrated turns still show read-tool summaries
- turns without plan or read calls do not render empty containers

## Implementation Notes

- keep the read-tool whitelist local to desktop for now: `read`, `glob`, `grep`
- prefer a small selector/helper in `chat/page.tsx` over introducing additional React state
- reuse existing argument and result summary formatting helpers where possible
- do not move plan logic into `PromptComposer`
- do not change Tauri command shapes, turn events, or persistence code

## Success Criteria

The design is complete when:

1. `update_plan` output is visible only as a running-turn floating card above the prompt input
2. assistant turns no longer include inline plan queue content
3. read tool activity appears as a compact, collapsed `Summary` task list inside the assistant context
4. the read task list shows only the latest 3 read tool calls
5. existing event protocol and persistence behavior remain unchanged
