# Desktop Prompt Composer Design

**Date:** 2026-03-07
**Status:** Approved
**Scope:** `desktop` prompt input surface for agent and workflow conversations

---

## 1. Background

The current chat route in `desktop/app/chat/page.tsx` is a placeholder, but the desktop frontend already has the primitives needed to build a new prompt surface:

- compact AI presentation styles under `desktop/components/ai`
- reusable UI atoms such as `InputGroup`, `DropdownMenu`, `Command`, and `Button`
- an existing visual language that is restrained, dense, and tool-oriented rather than chat-app styled

The official `ai-elements` prompt input can be used as a reference for API and interaction coverage, but ArgusX should not adopt its visual language directly. The new component needs to feel native to the desktop workbench.

## 2. Product Goal

Build a bottom-docked prompt composer that feels like a conversation box first, while still acting as the unified entry point for agent and workflow interactions.

The user-facing mental model is:

- the user is always talking to the system through one prompt box
- tools remain backend implementation detail
- the user may switch between `Agents` and `Workflows`
- the selector uses one unified selection model, but the menu is visually grouped into two categories

## 3. V1 Scope

### In Scope

- bottom-docked prompt composer
- multi-line text input
- explicit `Agents` and `Workflows` category controls
- grouped picker listing both categories
- current selection label and short description
- single submit action
- keyboard-first submit flow

### Out of Scope

- attachments
- slash commands
- `@` mentions
- exposed tool controls
- parameter panels
- session history UI
- full chat timeline redesign

## 4. UX Decision Summary

### Layout Model

Use a docked composer rather than a floating panel or centered landing card.

### Information Architecture

The component is a single continuous surface with two visible layers:

1. **Input layer**
   - a multi-line textarea
   - the primary visual focus
2. **Control layer**
   - bottom-left `Agents` and `Workflows` category controls
   - current selection label next to the category controls
   - lightweight selection description in the middle
   - one submit button on the right

### Category Model

- `Agents` and `Workflows` are always visible in the control layer
- they are not hidden inside the dropdown trigger
- they use clearly different accent colors
- switching category updates the current selection without clearing the draft

### Picker Model

- clicking the current selection opens one picker
- the picker always renders two groups: `Agents` and `Workflows`
- the active category has stronger emphasis
- the inactive group remains visible for quick switching

## 5. Visual Design

The component should feel like a professional workbench entry point, not a social chat composer.

### Surface

- fixed to the bottom of the main content region
- width constrained by the content container, not edge-to-edge full window
- one continuous card-like surface with clear top separation from the page
- stronger structure than a chat bubble, but softer than a command palette

### Textarea

- visually dominant
- default capacity should feel like two to three lines
- grows vertically until a max height, then scrolls internally
- generous padding so the surface feels deliberate rather than cramped

### Control Layer

- tighter than the input layer
- clearly reads as a secondary band
- supports quick scanning without competing with the input

### Color System

- `Agents`: cool accent family, preferably blue or teal
- `Workflows`: warm accent family, preferably amber, copper, or orange
- category accents should appear in labels, active states, and subtle tints
- category colors should not take over the full composer background
- submit button should keep the product primary color rather than reuse category accents

## 6. Component Architecture

Use a thin component family that matches the current `desktop/components` organization.

### Proposed Components

- `PromptComposer`
  - owns layout, submit behavior, keyboard rules, and controlled/uncontrolled support
- `PromptComposerTextarea`
  - owns textarea sizing and key handling
- `PromptComposerModeBar`
  - owns the bottom control band layout
- `PromptComposerCategoryTabs`
  - renders `Agents` and `Workflows` controls with active states
- `PromptComposerPicker`
  - renders the grouped selection popover
- `PromptComposerSubmit`
  - renders submit button states

### Suggested Placement

- reusable component: `desktop/components/ai/prompt-composer.tsx`
- tests colocated under `desktop/components/ai`
- minimal host surface under `desktop/app/chat` or a dev route

The implementation should reuse existing UI primitives instead of introducing a new control system.

## 7. State Model

Keep the state intentionally small.

### Core State

- `draft`: current text input
- `category`: `agent | workflow`
- `selectionId`: selected item id
- `status`: `idle | submitting | disabled`

### Rules

- changing `category` does not clear `draft`
- changing `selectionId` does not clear `draft`
- changing `category` should restore the last selection for that category when possible
- if no prior selection exists for a category, select its default first item
- `submitting` should lock the relevant controls consistently

## 8. Interaction Rules

### Submit

- `Enter` submits
- `Shift+Enter` inserts a newline
- submit is disabled for empty or whitespace-only drafts
- while submitting, the button shows loading and prevents duplicate submits

### Selection

- category switching is explicit and always available from the bottom-left
- the current selection label is adjacent to the category controls
- the picker groups items under `Agents` and `Workflows`
- unavailable items should be disabled in the picker with explanatory text

### Error Behavior

- failed submit should preserve the draft
- error feedback should appear adjacent to the composer, not in a blocking modal
- selection changes should not trigger confirmation dialogs

## 9. Accessibility and Testing Expectations

### Accessibility

- full keyboard navigation for composer and picker
- stable focus order across category tabs, picker trigger, textarea, and submit button
- appropriate `aria` semantics for grouped selection content
- visible focus styles consistent with the existing desktop UI

### Test Coverage

- `Enter` submit and `Shift+Enter` newline behavior
- empty draft disables submit
- switching category preserves draft
- switching selection preserves draft
- grouped picker renders both categories correctly
- disabled items cannot be selected
- submitting state prevents duplicate sends

## 10. Non-Goals and Guardrails

- Do not expose tools as first-class controls in this component.
- Do not add attachments or inline command systems in V1.
- Do not style the composer like a mobile messenger or consumer chat app.
- Do not overfit to the official `ai-elements` implementation; borrow interaction ideas only when they support the approved ArgusX design.

## 11. Implementation Direction

The implementation should optimize for:

- reuse of existing `desktop/components/ui` primitives
- minimal state surface
- clear separation between reusable composer logic and any page-specific host
- fast iteration via component tests before visual polish

This design is approved for planning and implementation handoff.
