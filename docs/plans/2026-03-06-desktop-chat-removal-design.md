# Desktop Chat Removal Design

Date: 2026-03-06
Status: Approved for planning
Scope: Remove desktop chat and LLM-specific implementation while preserving the existing application layout shell

## 1. Goal

Remove the current desktop `llm-chat` implementation and all user-facing chat or LLM capability signals, while preserving the current application layout:

- keep the shared layout shell, header, left navigation, and right sidebar behavior
- keep the `/chat` route as a static placeholder page
- replace dynamic chat UI with static placeholder content
- delete chat-specific frontend state, API bindings, Tauri commands, persistence, and runtime config code

## 2. Context

Current desktop behavior couples chat into three layers:

1. Route and layout
- `/chat` renders `ChatPage`
- non-chat routes mount the same `ChatPage` inside the right sidebar

2. Frontend chat implementation
- `desktop/components/features/chat/**` contains the entire chat UI
- `desktop/lib/stores/chat-*` owns session, turn, and message state
- `desktop/lib/api/chat.ts` binds the frontend to Tauri chat commands
- `desktop/lib/layout/chat-layout.ts` exists only to support chat-specific sizing

3. Tauri desktop runtime
- `desktop/src-tauri/src/lib.rs` registers chat session and turn commands
- `desktop/src-tauri/src/persistence/chat_repo.rs` persists chat messages and summaries
- `desktop/src-tauri/src/llm_runtime_config.rs` and related config paths exist only for chat runtime setup

The design target is to preserve the shell and route topology while removing the current chat product implementation.

## 3. Locked Decisions

1. Route policy
- keep `/chat`
- `/chat` becomes a static placeholder page and does not expose any working chat interactions

2. Layout policy
- keep `AppLayout`, left navigation, header, and right sidebar mechanics
- the right sidebar remains available on non-chat routes
- the right sidebar renders static placeholder content instead of the removed chat UI

3. Product messaging policy
- remove claims that ArgusX already provides LLM chat capability
- keep the navigation entry for `/chat`, but rename or rewrite surrounding copy so it reads as a placeholder module rather than an active feature

4. Removal policy
- delete chat-specific implementation rather than leaving dormant runtime code behind
- do not keep stores, Tauri commands, persistence, or configuration modules that exist only for the removed chat experience

## 4. Scope

### 4.1 Keep

- `desktop/components/layouts/**` shell structure
- generic `ui` components
- `desktop/app/layout.tsx`
- `desktop/app/annotation/page.tsx`
- generic styling tokens that are not chat-only
- `/chat` route entry point

### 4.2 Remove or replace

- `desktop/components/features/chat/**`
- `desktop/lib/stores/chat-*`
- `desktop/lib/api/chat.ts`
- `desktop/lib/layout/chat-layout.ts`
- chat-only tests under feature, store, and API folders
- Tauri chat command types and handlers
- chat persistence repository and related exports
- LLM runtime config state and commands that are only used by chat

## 5. Frontend Design

### 5.1 `/chat` page

The route remains reachable, but renders a static placeholder:

- title indicating the chat module has been removed
- short explanation that the area is reserved for the next redesign
- no composer, no message list, no session history, no model settings, no streaming state

The visual structure should still feel like the current app:

- use existing cards, badges, and spacing primitives
- preserve the full-height content region inside the existing layout shell
- avoid introducing fake activity, fake counters, or disabled chat controls that imply partially working behavior

### 5.2 Right sidebar placeholder

The right sidebar remains part of the layout on non-chat routes, but becomes a static information panel:

- small status card or description
- concise copy explaining that the conversation panel has been removed pending redesign
- no session switcher, prompt input, or runtime config actions

### 5.3 Home page and navigation copy

Chat and LLM-specific messaging is removed from the landing page:

- remove claims such as strong LLM chat capability, new chat, configure model, or AI agent conversation experience
- keep the `/chat` quick-access link only as an entry to the placeholder module
- use neutral wording such as module placeholder, redesign pending, or workspace reserved

## 6. Desktop Runtime Design

### 6.1 Tauri command surface

Remove commands and DTOs that exist only for chat:

- session create/list/update/delete
- message and turn summary loading
- agent turn start/cancel or stream plumbing if they are not used elsewhere in desktop
- runtime config read/write commands tied only to chat setup

### 6.2 App state and initialization

Trim `AppState` to dependencies still needed after chat removal.

Expected removals include:

- chat repository state
- runtime config repository state if only used by chat
- frontend/backed session mapping for chat turn handling
- agent stream emission plumbing that only serves the removed chat UI

### 6.3 Persistence

Remove desktop-local chat persistence:

- `chat_repo.rs`
- related exports from persistence modules
- bootstrap wiring for chat database setup

If any shared persistence module is still needed for non-chat features, keep it and narrow the public exports accordingly.

## 7. Testing and Verification

Frontend verification:

- `pnpm lint`
- `pnpm exec tsc --noEmit`
- targeted `vitest` runs for layout and app pages, updated to match the placeholder behavior

Rust verification:

- run `cargo test` for the desktop Tauri crate after removing chat-only modules
- ensure there are no remaining imports or command registrations referencing deleted chat code

Manual smoke checks:

- `/chat` renders static placeholder content
- non-chat routes still show the right sidebar and it opens without rendering removed chat UI
- left navigation and header remain unchanged structurally

## 8. Out of Scope

- designing the replacement chat experience
- adding a new domain module behind `/chat`
- changing annotation behavior
- changing the overall application layout system
- removing generic AI rendering components outside the current desktop chat removal scope unless they become dead references that block builds

## 9. Success Criteria

The change is complete when all of the following are true:

- `/chat` is reachable and renders a static placeholder
- the right sidebar still works as part of the layout shell
- desktop no longer ships chat session, message, or runtime-config behavior
- desktop UI copy no longer claims existing LLM chat capability
- TypeScript and Rust builds do not reference removed chat modules
