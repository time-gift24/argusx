# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ArgusX Desktop is a cross-platform desktop application for AI Agent management with LLM chat capabilities. It uses **Tauri v2** (Rust backend) + **Next.js 16** (React frontend with static export) + **Tailwind CSS v4** + **shadcn/ui**.

This project is part of a larger Rust workspace located at the parent directory (`../Cargo.toml`).

## Commands

```bash
# Development
pnpm dev                    # Start Next.js dev server (localhost:3000)
pnpm tauri dev              # Run Tauri app in development mode

# Build
pnpm build                  # Build Next.js static export to ../out
pnpm tauri build            # Build production Tauri application

# Lint
pnpm lint                   # Run ESLint
```

## Architecture

### Frontend-Backend Communication

The app uses **Tauri IPC** for frontend-backend communication:

1. **Rust backend** (`src-tauri/src/lib.rs`): Exposes `#[tauri::command]` functions
2. **TypeScript frontend** (`app/**`, `components/**`): Calls `invoke()` directly or via feature API wrappers

When adding new Tauri commands:
1. Add the command in `src-tauri/src/lib.rs` with `#[tauri::command]`
2. Register it in `invoke_handler(tauri::generate_handler![...])`
3. Add corresponding frontend typing and caller

### Workspace Structure

```
../                          # Rust workspace root
├── agent-core/              # Core agent logic
├── argusx-common/           # Shared config and utilities
├── bigmodel-api/            # API integrations
├── llm-sdk/                 # LLM SDK
├── llm-cli/                 # LLM CLI tool
└── argusx-desktop/          # This project (Tauri + Next.js)
```

### Key Directories

```
app/                         # Next.js App Router pages
components/
  ui/                        # shadcn/ui components (radix-mira style)
  layouts/                   # App layout, sidebars, theme toggle
  features/                  # Feature-specific components
lib/
  utils.ts                   # cn() helper for Tailwind
hooks/                       # Custom React hooks
src-tauri/                   # Rust backend
  src/lib.rs                 # Tauri commands and app setup
  tauri.conf.json            # Tauri configuration
```

### UI Conventions

- **shadcn/ui** with `radix-mira` style and `lucide` icons
- **Tailwind CSS v4** with oklch color system (see `app/globals.css`)
- Path aliases: `@/components`, `@/lib`, `@/hooks`
- Use `cn()` from `@/lib/utils` for conditional class merging

### Coding Practices

- **优先使用本地组件**: 先在 `components/` 目录下查找现有组件，尤其是 `components/layouts/` 和 `components/features/` 中的组件
- **使用 shadcn/ui 组件**: 当需要 UI 组件时，使用 shadcn 组件库，禁止引入其他前端库
- **禁止新建组件**: 未经用户明确允许，禁止在 `components/` 目录下创建新组件
- **路径别名**: 使用 `@/components`、`@/lib`、`@/hooks` 等路径别名
