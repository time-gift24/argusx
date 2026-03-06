# Desktop SOP Route Design

Date: 2026-03-06
Status: Approved for implementation
Scope: Move desktop SOP annotation entry from `/annotation` to `/sop/annotation` and express the nested route in the main content header without changing the flat left navigation

## 1. Goal

Make desktop SOP routing and UI semantics consistent:

- remove `/annotation`
- keep only `/sop/annotation`
- keep the left sidebar flat, with no new nesting
- express SOP hierarchy in the center content header with a breadcrumb
- use a `...` dropdown in the breadcrumb trail so future SOP pages have a clear expansion point

## 2. Locked Decisions

1. Route policy
- delete the old route entry at `/annotation`
- add the route entry at `/sop/annotation`
- do not keep a redirect or compatibility alias

2. Navigation policy
- left navigation remains flat
- there is a single SOP-related navigation item
- that item points directly to `/sop/annotation`

3. Header policy
- the annotation page keeps its existing workspace layout
- the page header changes from plain title text to a breadcrumb plus title/description block
- breadcrumb structure is:
  - `工作台`
  - `SOP`
  - `...` dropdown
  - `标注`

4. Dropdown policy
- the ellipsis is interactive, not decorative
- it should use the existing dropdown-menu and breadcrumb primitives
- it should stay minimal and only expose valid, current or clearly reserved destinations

## 3. Scope

### 3.1 Files to update

- `desktop/app/annotation/page.tsx`
- `desktop/app/sop/annotation/page.tsx`
- `desktop/app/page.tsx`
- `desktop/components/layouts/sidebar/app-sidebar.tsx`
- `desktop/components/features/annotation/annotation-page.tsx`
- related tests for homepage, sidebar, and annotation header

### 3.2 Files to keep unchanged

- `desktop/components/features/annotation/**` workspace internals
- `desktop/lib/annotation/**`
- annotation stores and reducers
- left sidebar layout mechanics

## 4. UX Notes

- The left nav should read as one flat list: dashboard, chat placeholder, SOP annotation.
- The main view should carry the hierarchy signal, not the nav.
- The breadcrumb dropdown must remain compact and neutral; it is there to preserve information scent, not to advertise nonexistent modules.

## 5. Success Criteria

- `/annotation` is no longer a desktop route entry
- `/sop/annotation` renders the current annotation experience
- left nav contains exactly one SOP-related entry and it points to `/sop/annotation`
- the annotation page header shows a breadcrumb with an ellipsis dropdown in the middle
- homepage links no longer point to `/annotation`
