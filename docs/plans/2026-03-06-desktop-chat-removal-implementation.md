# Desktop Chat Removal Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the current desktop chat and LLM-specific implementation while preserving the existing layout shell and keeping `/chat` as a static placeholder route.

**Architecture:** Replace the dynamic chat surface with a small shared placeholder component, then delete all frontend and Tauri layers that only exist to support the removed chat workflow. Keep the application shell, navigation topology, and right-sidebar mechanics intact, but make them render neutral placeholder content instead of live chat state.

**Tech Stack:** Next.js 16, React 19, TypeScript, Vitest, Tauri 2, Rust

---

### Task 1: Replace Chat Surfaces With Static Placeholders

**Files:**
- Create: `desktop/components/placeholders/chat-module-placeholder.tsx`
- Create: `desktop/components/placeholders/chat-module-placeholder.test.tsx`
- Create: `desktop/components/layouts/sidebar/module-sidebar.tsx`
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/components/layouts/app-layout.tsx`
- Modify: `desktop/components/layouts/sidebar/index.ts`
- Modify: `desktop/components/ui/sidebar.tsx`
- Delete: `desktop/components/layouts/sidebar/chat-sidebar.tsx`
- Delete: `desktop/lib/layout/chat-layout.ts`

**Step 1: Write the failing tests**

```tsx
import { render, screen } from "@testing-library/react";
import { ChatModulePlaceholder } from "./chat-module-placeholder";

describe("ChatModulePlaceholder", () => {
  it("renders the redesign placeholder copy for the page surface", () => {
    render(<ChatModulePlaceholder variant="page" />);
    expect(screen.getByText("对话模块已移除")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("renders compact placeholder copy for the sidebar surface", () => {
    render(<ChatModulePlaceholder variant="sidebar" />);
    expect(screen.getByText("右侧面板占位")).toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run desktop/components/placeholders/chat-module-placeholder.test.tsx`

Expected: FAIL because `ChatModulePlaceholder` does not exist yet.

**Step 3: Write the minimal implementation**

Create one shared placeholder component with a `variant` prop:

```tsx
type ChatModulePlaceholderProps = {
  variant: "page" | "sidebar";
};

export function ChatModulePlaceholder({ variant }: ChatModulePlaceholderProps) {
  if (variant === "sidebar") {
    return <div>Right sidebar placeholder</div>;
  }

  return <div>Chat module placeholder page</div>;
}
```

Then wire it into:

- `desktop/app/chat/page.tsx`
- `desktop/components/layouts/sidebar/module-sidebar.tsx`
- `desktop/components/layouts/app-layout.tsx`
- `desktop/components/layouts/sidebar/index.ts`

In `desktop/components/ui/sidebar.tsx`, replace the import of `CHAT_SIDEBAR_MIN_WIDTH` with a local generic right-sidebar minimum width constant.

**Step 4: Run tests and typecheck**

Run: `pnpm --dir desktop vitest run desktop/components/placeholders/chat-module-placeholder.test.tsx`

Expected: PASS

Run: `pnpm --dir desktop exec tsc --noEmit`

Expected: FAIL later on remaining chat imports, but no new failures from the placeholder files themselves.

**Step 5: Commit**

```bash
git add desktop/components/placeholders/chat-module-placeholder.tsx \
  desktop/components/placeholders/chat-module-placeholder.test.tsx \
  desktop/components/layouts/sidebar/module-sidebar.tsx \
  desktop/app/chat/page.tsx \
  desktop/components/layouts/app-layout.tsx \
  desktop/components/layouts/sidebar/index.ts \
  desktop/components/ui/sidebar.tsx
git commit -m "refactor(desktop): replace chat surfaces with placeholder shell"
```

### Task 2: Remove Chat and LLM Messaging From Navigation And Dashboard

**Files:**
- Modify: `desktop/components/layouts/sidebar/app-sidebar.tsx`
- Modify: `desktop/app/page.tsx`
- Modify: `desktop/app/layout.tsx`
- Create: `desktop/app/page.test.tsx`

**Step 1: Write the failing test**

```tsx
import { render, screen } from "@testing-library/react";
import DashboardPage from "./page";

describe("DashboardPage", () => {
  it("does not advertise live LLM chat capability", () => {
    render(<DashboardPage />);
    expect(
      screen.queryByText(/LLM对话能力|开始新对话|配置您的模型|AI Agent 交互体验/)
    ).not.toBeInTheDocument();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run desktop/app/page.test.tsx`

Expected: FAIL because the existing dashboard copy still mentions live chat and LLM capability.

**Step 3: Write the minimal implementation**

Update:

- left navigation copy in `desktop/components/layouts/sidebar/app-sidebar.tsx`
- dashboard cards and quick links in `desktop/app/page.tsx`
- app metadata description in `desktop/app/layout.tsx`

Keep the `/chat` link, but rewrite copy so it clearly points to a placeholder module awaiting redesign.

**Step 4: Run tests**

Run: `pnpm --dir desktop vitest run desktop/app/page.test.tsx`

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/components/layouts/sidebar/app-sidebar.tsx \
  desktop/app/page.tsx \
  desktop/app/layout.tsx \
  desktop/app/page.test.tsx
git commit -m "refactor(desktop): remove chat and llm product messaging"
```

### Task 3: Delete Frontend Chat Runtime, Stores, And Dead AI Rendering Modules

**Files:**
- Modify: `desktop/app/globals.css`
- Modify: `desktop/lib/api/index.ts`
- Modify: `desktop/package.json`
- Delete: `desktop/components/features/chat/*`
- Delete: `desktop/components/ai/*`
- Delete: `desktop/components/ai-elements/*`
- Delete: `desktop/lib/api/chat.ts`
- Delete: `desktop/lib/api/chat.test.ts`
- Delete: `desktop/lib/stores/chat-store.ts`
- Delete: `desktop/lib/stores/chat-store-load-turns.test.ts`
- Delete: `desktop/lib/stores/chat-store-scroll-signal.test.ts`
- Delete: `desktop/lib/stores/chat-store-update-plan.test.ts`
- Delete: `desktop/lib/stores/chat-selectors.ts`
- Delete: `desktop/lib/stores/chat-selectors.test.ts`
- Delete: `desktop/lib/stores/chat-cache-budget.ts`
- Delete: `desktop/lib/stores/chat-cache-budget.test.ts`
- Delete: `desktop/lib/stores/llm-runtime-config-store.ts`
- Delete: `desktop/app/styles/streamdown.css`
- Delete: `desktop/app/styles/streamdown.test.ts`
- Optional cleanup: `desktop/TEST_CASES.md`, `desktop/components/features/index.ts`, any dead README files referencing removed chat modules

**Step 1: Run the failing integration check**

Run: `pnpm --dir desktop exec tsc --noEmit`

Expected: FAIL with import errors pointing at deleted or still-referenced chat modules.

**Step 2: Remove the implementation and clean imports**

Delete the entire frontend chat surface and remove any remaining imports or endpoints that reference it.

Minimum required cleanups:

- remove `chat` from `desktop/lib/api/index.ts`
- remove `streamdown.css` import from `desktop/app/globals.css`
- prune chat-only dependencies from `desktop/package.json` if they are no longer imported anywhere
- delete dead `components/ai` and `components/ai-elements` modules if they are only serving the removed chat feature

**Step 3: Re-run the integration check**

Run: `pnpm --dir desktop exec tsc --noEmit`

Expected: Either PASS or fail only on Tauri-side invoke typings or route imports not yet cleaned in Task 4.

Run: `pnpm --dir desktop lint`

Expected: Either PASS or report only issues caused by the remaining Rust-side/Tauri cleanup work.

**Step 4: Commit**

```bash
git add desktop/app/globals.css desktop/lib/api/index.ts desktop/package.json desktop
git commit -m "refactor(desktop): remove frontend chat runtime modules"
```

### Task 4: Remove Tauri Chat Commands, Persistence, And LLM Runtime Wiring

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/persistence/mod.rs`
- Modify: `desktop/src-tauri/src/persistence/schema.rs`
- Delete: `desktop/src-tauri/src/llm_runtime_config.rs`
- Delete: `desktop/src-tauri/src/system_prompt.rs`
- Delete: `desktop/src-tauri/src/persistence/chat_repo.rs`
- Delete: `desktop/src-tauri/src/persistence/runtime_config_repo.rs`
- Conditional modify: `Cargo.toml` at workspace root if deleted workspace members or path dependencies prevent desktop verification

**Step 1: Run the failing Rust verification**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: FAIL because chat-related modules, path dependencies, or workspace members still point to deleted runtime crates.

**Step 2: Write the minimal implementation**

Trim the desktop Tauri crate down to the commands and state still needed after chat removal.

Required deletions:

- chat session DTOs
- chat session/message/turn commands
- agent stream forwarding
- LLM runtime config load/save/clear commands
- chat repo and runtime config repo wiring
- `AppState` fields that only exist for the removed chat flow

Required manifest cleanup:

- remove path dependencies on deleted chat/runtime crates from `desktop/src-tauri/Cargo.toml`
- if `cargo` still fails before compiling because the workspace root lists deleted members, update the root `Cargo.toml` workspace `members` list to only include crates that still exist

Keep any truly non-chat desktop functionality that still has a caller, such as generic shell startup or cookie-gateway features, but delete dead code aggressively.

**Step 3: Re-run Rust verification**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: PASS, or fail only on unrelated pre-existing workspace breakage that is outside the edited files and can be explicitly documented.

**Step 4: Commit**

```bash
git add Cargo.toml desktop/src-tauri/Cargo.toml desktop/src-tauri/src
git commit -m "refactor(desktop): remove tauri chat runtime wiring"
```

### Task 5: Full Verification And Cleanup Pass

**Files:**
- Review all modified files from Tasks 1-4

**Step 1: Run frontend verification**

Run: `pnpm --dir desktop test`

Expected: PASS

Run: `pnpm --dir desktop lint`

Expected: PASS

Run: `pnpm --dir desktop exec tsc --noEmit`

Expected: PASS

**Step 2: Run Rust verification**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: PASS, unless unrelated workspace deletions still block the build and have been documented explicitly.

**Step 3: Manual smoke checklist**

1. Open `/chat` and confirm it renders only placeholder content.
2. Open a non-chat route and confirm the right sidebar still opens.
3. Confirm the right sidebar no longer renders message lists, prompt input, model configuration, or streaming state.
4. Confirm dashboard and nav copy no longer advertises live chat or LLM capability.

**Step 4: Final commit**

```bash
git add docs/plans/2026-03-06-desktop-chat-removal-design.md \
  docs/plans/2026-03-06-desktop-chat-removal-implementation.md \
  desktop \
  Cargo.toml
git commit -m "refactor(desktop): remove legacy chat module"
```
