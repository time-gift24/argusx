# Desktop Dev Hub Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a first-class `/dev` overview page to the desktop app, expose it as a separate sidebar group, and extend the shared header breadcrumb to support the Dev area.

**Architecture:** Keep the implementation intentionally small. Add one new overview route at `desktop/app/dev/page.tsx`, drive its left-directory and right-detail content from a declarative metadata file, and keep existing playground routes untouched. Update the existing `AppSidebar` and `AppLayout` rather than introducing new layout shells or nested dev routing.

**Tech Stack:** Next.js 16, React 19, TypeScript, Vitest, Testing Library, existing desktop layout and sidebar primitives

---

### Task 1: Create the Dev Hub metadata and default page rendering

**Files:**
- Create: `desktop/lib/dev-showcases.ts`
- Create: `desktop/app/dev/page.tsx`
- Create: `desktop/app/dev/page.test.tsx`

**Step 1: Write the failing test**

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import DevPage from "./page";

describe("DevPage", () => {
  it("renders the dev directory and defaults to the prompt composer showcase", () => {
    render(<DevPage />);

    expect(screen.getByRole("heading", { level: 1, name: "Dev" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Prompt Composer" })).toHaveAttribute(
      "data-state",
      "active"
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "Prompt Composer" })
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open showcase/i })).toHaveAttribute(
      "href",
      "/chat"
    );
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run app/dev/page.test.tsx`

Expected: FAIL because `desktop/app/dev/page.tsx` and the metadata source do not exist yet.

**Step 3: Write minimal implementation**

Create `desktop/lib/dev-showcases.ts` with a small ordered metadata list:

```ts
export const DEV_SHOWCASES = [
  {
    id: "prompt-composer",
    title: "Prompt Composer",
    href: "/chat",
    status: "Available",
    summary: "Explore the docked prompt composer with agent and workflow targeting.",
    highlights: [
      "Agent and workflow switching",
      "Grouped picker behavior",
      "Async submit and retry state",
    ],
    order: 0,
  },
  {
    id: "stream",
    title: "Stream Playground",
    href: "/dev/stream",
    status: "Available",
    summary: "Inspect runtime stream surfaces and open/close behavior.",
    highlights: ["Reasoning surface", "Tool state transitions"],
    order: 1,
  },
  {
    id: "streamdown",
    title: "Streamdown Playground",
    href: "/dev/streamdown",
    status: "Available",
    summary: "Inspect markdown, code, math, and mermaid rendering.",
    highlights: ["Markdown", "Code panels", "Mermaid"],
    order: 2,
  },
] as const;
```

Create the minimal page:

```tsx
"use client";

import Link from "next/link";
import { useState } from "react";
import { DEV_SHOWCASES } from "@/lib/dev-showcases";

export default function DevPage() {
  const [selectedId, setSelectedId] = useState("prompt-composer");
  const selected =
    DEV_SHOWCASES.find((item) => item.id === selectedId) ?? DEV_SHOWCASES[0];

  return (
    <div className="grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)]">
      <section>
        <h1>Dev</h1>
        {DEV_SHOWCASES.map((item) => (
          <button
            data-state={item.id === selected.id ? "active" : "inactive"}
            key={item.id}
            onClick={() => setSelectedId(item.id)}
            type="button"
          >
            {item.title}
          </button>
        ))}
      </section>
      <section>
        <h2>{selected.title}</h2>
        <p>{selected.summary}</p>
        <Link href={selected.href}>Open showcase</Link>
      </section>
    </div>
  );
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run app/dev/page.test.tsx`

Expected: PASS with the default Dev Hub rendering assertions green.

**Step 5: Commit**

```bash
git add desktop/lib/dev-showcases.ts desktop/app/dev/page.tsx desktop/app/dev/page.test.tsx
git commit -m "feat: add dev hub overview page"
```

### Task 2: Add the full dual-panel detail behavior

**Files:**
- Modify: `desktop/lib/dev-showcases.ts`
- Modify: `desktop/app/dev/page.tsx`
- Modify: `desktop/app/dev/page.test.tsx`

**Step 1: Write the failing test**

```tsx
import userEvent from "@testing-library/user-event";

it("switches the right panel when a different showcase is selected", async () => {
  const user = userEvent.setup();
  render(<DevPage />);

  await user.click(screen.getByRole("button", { name: "Streamdown Playground" }));

  expect(
    screen.getByRole("heading", { level: 2, name: "Streamdown Playground" })
  ).toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: /open showcase/i })
  ).toHaveAttribute("href", "/dev/streamdown");
  expect(screen.getByText("Markdown")).toBeInTheDocument();
  expect(screen.getByText("Mermaid")).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run app/dev/page.test.tsx`

Expected: FAIL because the page does not yet render showcase highlights or richer detail content.

**Step 3: Write minimal implementation**

Extend the metadata shape:

```ts
type DevShowcase = {
  id: string;
  title: string;
  href: string;
  status: "Available" | "Experimental";
  summary: string;
  details: string;
  highlights: string[];
  order: number;
};
```

Expand the right panel:

```tsx
<section className="rounded-xl border border-border/60 bg-card p-5">
  <div className="flex items-center gap-2">
    <h2 className="text-xl font-semibold">{selected.title}</h2>
    <span className="rounded-full border px-2 py-0.5 text-[10px] uppercase">
      {selected.status}
    </span>
  </div>
  <p className="mt-2 text-sm text-muted-foreground">{selected.summary}</p>
  <p className="mt-4 text-sm leading-6">{selected.details}</p>
  <ul className="mt-4 space-y-1 text-sm">
    {selected.highlights.map((item) => (
      <li key={item}>{item}</li>
    ))}
  </ul>
  <div className="mt-6">
    <Link href={selected.href}>Open showcase</Link>
  </div>
</section>
```

Style the left directory as compact tool-style list buttons, not large cards.

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run app/dev/page.test.tsx`

Expected: PASS with selection switching and detail rendering green.

**Step 5: Commit**

```bash
git add desktop/lib/dev-showcases.ts desktop/app/dev/page.tsx desktop/app/dev/page.test.tsx
git commit -m "feat: add dev hub detail panel"
```

### Task 3: Add the Dev sidebar group at the bottom

**Files:**
- Modify: `desktop/components/layouts/sidebar/app-sidebar.tsx`
- Modify: `desktop/components/layouts/sidebar/app-sidebar.test.tsx`

**Step 1: Write the failing test**

```tsx
it("renders a separate Dev group below the workspace group", () => {
  mockPathname = "/dev";

  const { container } = render(
    <SidebarProvider>
      <AppSidebar />
    </SidebarProvider>
  );

  const labels = Array.from(
    container.querySelectorAll('[data-slot="sidebar-group-label"]')
  ).map((element) => element.textContent);

  expect(labels).toEqual(["工作区", "Dev"]);
  expect(screen.getByRole("link", { name: /^dev$/i })).toHaveAttribute(
    "href",
    "/dev"
  );
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run desktop/components/layouts/sidebar/app-sidebar.test.tsx`

Expected: FAIL because the sidebar only has the workspace group today.

**Step 3: Write minimal implementation**

Introduce a second navigation array and render a second `SidebarGroup` after the existing workspace group:

```tsx
const navDev = [
  {
    title: "Dev",
    url: "/dev",
    icon: FlaskConical,
  },
];
```

Render:

```tsx
<SidebarGroup>
  <SidebarGroupLabel>Dev</SidebarGroupLabel>
  <SidebarMenu>
    {navDev.map((item) => (
      <SidebarMenuItem key={item.title}>
        <SidebarMenuButton asChild isActive={pathname === item.url}>
          <Link href={item.url}>...</Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    ))}
  </SidebarMenu>
</SidebarGroup>
```

Keep the `Dev` group below `工作区`.

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run desktop/components/layouts/sidebar/app-sidebar.test.tsx`

Expected: PASS with the group ordering and Dev link assertions green.

**Step 5: Commit**

```bash
git add desktop/components/layouts/sidebar/app-sidebar.tsx desktop/components/layouts/sidebar/app-sidebar.test.tsx
git commit -m "feat: add dev group to sidebar"
```

### Task 4: Extend the shared breadcrumb to cover /dev

**Files:**
- Modify: `desktop/components/layouts/app-layout.tsx`
- Modify: `desktop/components/layouts/app-layout.test.tsx`

**Step 1: Write the failing test**

```tsx
it("shows the Dev breadcrumb on the dev overview route", () => {
  mockPathname = "/dev";

  render(
    <AppLayout>
      <div>workspace</div>
    </AppLayout>
  );

  expect(screen.getByText("工作台")).toBeInTheDocument();
  expect(screen.getByText("Dev")).toBeInTheDocument();
  expect(
    screen.queryByLabelText("打开 SOP 页面导航")
  ).not.toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run desktop/components/layouts/app-layout.test.tsx`

Expected: FAIL because `RouteBreadcrumb` currently only handles `/sop/annotation`.

**Step 3: Write minimal implementation**

Extend `RouteBreadcrumb`:

```tsx
function RouteBreadcrumb({ pathname }: { pathname: string }) {
  if (pathname === "/dev") {
    return (
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>
            <BreadcrumbLink asChild>
              <Link href="/">工作台</Link>
            </BreadcrumbLink>
          </BreadcrumbItem>
          <BreadcrumbSeparator />
          <BreadcrumbItem>
            <BreadcrumbPage>Dev</BreadcrumbPage>
          </BreadcrumbItem>
        </BreadcrumbList>
      </Breadcrumb>
    );
  }

  if (pathname !== "/sop/annotation") {
    return null;
  }

  // existing SOP breadcrumb
}
```

Keep the existing chat-route behavior unchanged.

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run desktop/components/layouts/app-layout.test.tsx`

Expected: PASS with both the SOP and Dev breadcrumb cases green.

**Step 5: Commit**

```bash
git add desktop/components/layouts/app-layout.tsx desktop/components/layouts/app-layout.test.tsx
git commit -m "feat: add dev breadcrumb"
```

### Task 5: Run verification and prepare the branch for implementation review

**Files:**
- Modify: `desktop/app/dev/page.tsx`
- Modify: `desktop/app/dev/page.test.tsx`
- Modify: `desktop/components/layouts/sidebar/app-sidebar.tsx`
- Modify: `desktop/components/layouts/sidebar/app-sidebar.test.tsx`
- Modify: `desktop/components/layouts/app-layout.tsx`
- Modify: `desktop/components/layouts/app-layout.test.tsx`
- Create: `desktop/lib/dev-showcases.ts`

**Step 1: Run the targeted route and shell tests**

Run:

```bash
pnpm --dir desktop exec vitest run \
  app/dev/page.test.tsx \
  components/layouts/sidebar/app-sidebar.test.tsx \
  components/layouts/app-layout.test.tsx
```

Expected: PASS for the Dev Hub page, sidebar group, and breadcrumb behavior.

**Step 2: Run the full desktop suite**

Run: `pnpm --dir desktop test`

Expected: PASS with the full desktop suite green.

**Step 3: Run lint on touched files**

Run:

```bash
pnpm --dir desktop exec eslint --no-warn-ignored \
  app/dev/page.tsx \
  app/dev/page.test.tsx \
  components/layouts/sidebar/app-sidebar.tsx \
  components/layouts/sidebar/app-sidebar.test.tsx \
  components/layouts/app-layout.tsx \
  components/layouts/app-layout.test.tsx \
  lib/dev-showcases.ts
```

Expected: PASS with no lint errors on the touched implementation files.

**Step 4: Review the diff before committing**

Run:

```bash
git diff --stat
git diff -- app/dev/page.tsx components/layouts/sidebar/app-sidebar.tsx components/layouts/app-layout.tsx
```

Expected: only the planned Dev Hub, sidebar, and breadcrumb changes.

**Step 5: Commit**

```bash
git add desktop/app/dev/page.tsx desktop/app/dev/page.test.tsx desktop/components/layouts/sidebar/app-sidebar.tsx desktop/components/layouts/sidebar/app-sidebar.test.tsx desktop/components/layouts/app-layout.tsx desktop/components/layouts/app-layout.test.tsx desktop/lib/dev-showcases.ts
git commit -m "feat: add desktop dev hub"
```

### Notes

- Keep `Prompt Composer` linked to `/chat` in this pass; do not introduce a second dedicated prompt-composer dev route.
- Do not turn `Dev` into an expandable sidebar subtree.
- Keep the Dev Hub overview informational and navigational; do not embed the existing `stream` or `streamdown` pages inline.
