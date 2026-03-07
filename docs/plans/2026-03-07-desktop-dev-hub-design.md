# Desktop Dev Hub Design

**Date:** 2026-03-07
**Status:** Approved
**Scope:** `desktop` dev overview page, sidebar navigation, and header breadcrumb integration

---

## 1. Background

The desktop app already has isolated development playground routes under `desktop/app/dev`, but they are not surfaced through the main application navigation:

- `desktop/app/dev/stream`
- `desktop/app/dev/streamdown`

The left sidebar currently only exposes the main workspaces (`对话`, `SOP 标注`), and the header breadcrumb logic only handles `/sop/annotation`.

This makes the existing dev surfaces discoverable only by direct URL, and there is no unified overview page for browsing or choosing between available showcases.

## 2. Product Goal

Add a first-class `Dev` area to the desktop app that behaves like a curated internal showcase hub:

- discoverable from the main sidebar
- consistent with the existing application shell
- readable through the same header breadcrumb pattern used by the SOP annotation page
- centered on choosing what to inspect rather than embedding every demo inline

The user-facing mental model is:

- `Dev` is a dedicated area, not another everyday workspace
- `/dev` is the overview page
- individual playgrounds remain available as their own routes

## 3. V1 Scope

### In Scope

- new `/dev` overview page
- independent `Dev` sidebar group placed below the existing workspace group
- header breadcrumb support for `/dev`
- dual-panel layout on `/dev`
- showcase directory for the current development pages
- a new showcase entry for the prompt composer

### Out of Scope

- shared nested layout for all `/dev/*` routes
- inline live embedding of every playground into `/dev`
- sidebar expansion with multiple dev sub-items
- search, filtering, or tagging inside the dev hub
- restructuring existing `stream` and `streamdown` playground pages

## 4. UX Decision Summary

### Navigation Model

The sidebar should have two top-level groups:

1. `工作区`
   - existing workspace entries stay here
2. `Dev`
   - one entry only: `Dev`
   - this group is rendered below the workspace group

`Dev` is not mixed into the workspace list. It is visually and semantically separate.

### Route Model

Use `/dev` as the overview hub. Existing child routes remain intact:

- `/dev`
- `/dev/stream`
- `/dev/streamdown`
- future showcase routes such as `/dev/prompt-composer`

### Breadcrumb Model

Header breadcrumb behavior should match the existing SOP annotation treatment:

- `/dev` renders `工作台 / Dev`
- future detailed showcase routes can expand this to `工作台 / Dev / <Page>`
- breadcrumb rendering remains centralized in `AppLayout`

## 5. Page Layout

`/dev` should be a dual-panel workbench page.

### Left Panel

The left panel acts as the directory:

- fixed list of showcase entries
- compact tool-style list items rather than large landing-page cards
- current selection is visually emphasized
- manual ordering only

### Right Panel

The right panel acts as the detail view for the selected showcase:

- title
- short purpose statement
- what the showcase is useful for
- a small list of key behaviors or verification points
- lightweight status badge, such as `Available` or `Experimental`
- primary button to open the full showcase page

### Interaction Model

- selecting a showcase from the left panel only updates the right panel detail view
- the URL stays `/dev`
- the user chooses whether to enter the dedicated showcase route
- `/dev` defaults to the prompt composer showcase as the primary entry

## 6. Content Strategy

The `/dev` hub is not a router replacement and not a mega-preview page.

It should provide:

- curation
- orientation
- entry points

It should not provide:

- full embedded reproductions of each playground
- multiple scroll-heavy demos stacked on one page
- a second navigation system inside the main navigation

This keeps the hub easy to scan and avoids mixing overview and experimentation concerns.

## 7. Component and Data Design

### Suggested Structure

- `desktop/app/dev/page.tsx`
  - entry for the Dev Hub page
- optional local component extraction if the page becomes long
- `desktop/lib/dev-showcases.ts`
  - source of truth for showcase metadata

### Showcase Metadata Shape

The data source should define each dev item with fields similar to:

- `id`
- `title`
- `slug`
- `href`
- `summary`
- `details`
- `status`
- `highlights`
- `order`

This keeps the hub view declarative and allows future showcase items to be added without rewriting layout logic.

## 8. Application Shell Changes

### Sidebar

Modify the left sidebar to:

- keep the existing `工作区` group unchanged
- add a new `Dev` group below it
- place one link to `/dev` inside that group

### App Layout

Extend the route-aware breadcrumb logic so it recognizes `/dev`.

The implementation should preserve the current behavior:

- no breadcrumb on chat routes
- SOP breadcrumb on `/sop/annotation`
- Dev breadcrumb on `/dev`

## 9. Testing Expectations

### New Coverage

- `/dev` page renders left directory and right detail panel
- default selection on `/dev` is the prompt composer showcase
- right panel contains a button to open the detailed showcase route
- `/dev` route shows the expected breadcrumb

### Updated Coverage

- sidebar test confirms `Dev` is a separate group
- sidebar test confirms the `Dev` group appears below `工作区`

## 10. Guardrails

- Do not add dev sub-pages as expanded sidebar children in this pass.
- Do not replace existing playground routes with the new hub.
- Do not embed full playground implementations into `/dev`.
- Keep the visual language aligned with the existing desktop shell, not a marketing or docs-site aesthetic.

## 11. Implementation Direction

The implementation should prioritize:

- minimal shell changes
- a single source of truth for showcase metadata
- easy extension for future dev pages
- consistency with the current breadcrumb and sidebar architecture

This design is approved for implementation planning.
