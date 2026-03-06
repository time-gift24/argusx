# Desktop Large-Screen Layout Design

Date: 2026-03-06
Status: Approved for implementation
Scope: Rebalance the desktop shell for 27-inch and 31.5-inch displays, make `/` render the chat workspace directly, and reduce the left navigation to a secondary aid

## 1. Goal

Shift the desktop app from a portal-style landing page to a default workspace layout:

- make `/` render the same chat placeholder workspace as `/chat`
- remove the dashboard-style homepage role from the product
- reduce the left sidebar so it behaves like a light navigation aid
- expand the center workspace so large desktop displays are used by default
- widen the SOP annotation workspace so the review pane is clearly dominant on large screens

## 2. Locked Decisions

1. Default entry policy
- `/` renders the chat page content directly
- `/chat` remains available as an explicit route
- the app no longer opens to a dashboard-style summary page

2. Left navigation policy
- remove the dashboard nav item
- keep the sidebar flat
- keep only lightweight primary entries:
  - `对话`
  - `SOP 标注`
- reduce the default and minimum left sidebar widths so it reads as auxiliary chrome instead of a content column

3. Main workspace policy
- prefer full-width desktop content instead of centered narrow cards
- keep the current header and sidebar mechanics
- avoid introducing extra route hierarchy or new modules

4. SOP workspace policy
- keep the current annotation feature structure
- widen the overall review layout for large displays
- increase the right detail panel width modestly so form work remains comfortable while the review pane still dominates

## 3. Scope

### 3.1 Files to update

- `desktop/app/page.tsx`
- `desktop/app/page.test.tsx`
- `desktop/app/chat/page.tsx`
- `desktop/components/placeholders/chat-module-placeholder.tsx`
- `desktop/components/placeholders/chat-module-placeholder.test.tsx`
- `desktop/components/layouts/app-layout.tsx`
- `desktop/components/layouts/sidebar/app-sidebar.tsx`
- `desktop/components/layouts/sidebar/app-sidebar.test.tsx`
- `desktop/components/ui/sidebar.tsx`
- `desktop/components/features/annotation/annotation-workspace.tsx`

### 3.2 Files to keep unchanged

- right sidebar open/close mechanics
- SOP breadcrumb structure already added in `annotation-page.tsx`
- annotation data flow, stores, and field components

## 4. UX Notes

- The first frame after app launch should already feel like the main work surface.
- Left navigation should remain available, but visually defer to the central workspace.
- Chat placeholder content should read like a workspace shell, not a modal card dropped into empty space.
- The SOP review pane should gain noticeably more horizontal room on large monitors without turning the right form into an oversized drawer.

## 5. Success Criteria

- `/` renders the same chat workspace content as `/chat`
- the left sidebar no longer exposes a dashboard entry
- the left sidebar defaults to a narrower width than today
- the chat placeholder page uses a wider desktop-oriented layout
- the SOP annotation workspace uses a wider right panel than the previous `360px` column
