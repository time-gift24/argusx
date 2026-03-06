# Desktop Fixed Header Scroll Design

Date: 2026-03-06
Status: Approved for implementation
Scope: Keep the global desktop header fixed, move page breadcrumb into the header, and make only the main content region scroll

## 1. Goal

Correct the desktop shell so the page body no longer grows past the viewport:

- keep the global `ArgusX` header fixed
- make the area below the header a fixed-height scroll container
- move page breadcrumb from the SOP page body into the global header
- show the second breadcrumb row only on routes other than `/` and `/chat`

## 2. Locked Decisions

1. Scroll policy
- the app shell still owns the viewport height
- the header does not scroll
- the main content region below the header becomes the only vertical scroll container
- page components should not own full-page scrolling

2. Header policy
- the header keeps `ArgusX` on the first row
- on `/` and `/chat`, the header stays single-line
- on other routes, the header adds a second row below `ArgusX` for breadcrumb

3. Breadcrumb policy
- breadcrumb rendering moves from `annotation-page.tsx` into `app-layout.tsx`
- the existing `...` dropdown stays in the breadcrumb
- current supported breadcrumb output is only what the app already needs now:
  - `/sop/annotation` => `SOP / ... / 标注`

## 3. Scope

### 3.1 Files to update

- `desktop/components/layouts/app-layout.tsx`
- `desktop/components/layouts/app-layout.test.tsx`
- `desktop/components/features/annotation/annotation-page.tsx`
- `desktop/components/features/annotation/annotation-page.test.tsx`

### 3.2 Files to keep unchanged

- sidebar open/close mechanics
- root and chat route shell logic
- SOP annotation workspace structure

## 4. UX Notes

- The fixed header should feel like desktop chrome, not page content.
- Main content scroll must start immediately below the header line.
- Breadcrumb should stay visually secondary to the `ArgusX` title.
- `/` and `/chat` should remain visually quiet and not show extra hierarchy.

## 5. Success Criteria

- the main content region below the header is the scroll container
- the header remains fixed while content scrolls
- `/sop/annotation` shows breadcrumb in the global header, under `ArgusX`
- `AnnotationPage` no longer renders breadcrumb inside the page body
- `/` and `/chat` do not show the header breadcrumb row
