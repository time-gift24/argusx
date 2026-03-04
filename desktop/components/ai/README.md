# AI Components Module

This module provides centralized, deterministic AI messaging rendering components for the ArgusX Desktop chat interface.

## Design Principles

1. **Deterministic Rendering**: All code highlighting is controlled by a centralized policy (`highlight-policy.ts`)
2. **Unified Runtime Surface**: Consistent visual styling across code blocks, terminal output, and process sections
3. **Theme-Aware**: CSS variables ensure proper rendering in both light and dark themes

## Components

### Core Rendering
- `message.tsx` - Message containers and response rendering
- `reasoning.tsx` - Collapsible reasoning/thinking blocks
- `plan.tsx` - Structured plan/action displays
- `tool.tsx` - Tool invocation and result displays
- `terminal.tsx` - Terminal output surfaces
- `code-block.tsx` - Syntax-highlighted code blocks

### Policy & Styling
- `highlight-policy.ts` - Deterministic language allowlist for syntax highlighting
- `styles.ts` - Centralized runtime surface style tokens
- `streamdown-code.tsx` - Custom Streamdown code renderer with policy integration
- `streamdown-plugins.ts` - Streamdown plugin bundle

## Usage

```tsx
// Import from centralized location
import {
  Message,
  MessageResponse,
  CodeBlock,
  shouldHighlightFence,
  getRuntimeSurfaceClass
} from "@/components/ai";

// Check if a language should be highlighted
const highlighted = shouldHighlightFence({
  isFenced: true,
  language: "typescript"
});

// Get runtime surface class
const surfaceClass = getRuntimeSurfaceClass("code");
```

## Highlighting Policy

Syntax highlighting is **deterministic** and **allowlist-based**:

**Allowed languages:**
- rust, typescript, ts, javascript, js
- tsx, jsx, python, py
- go, java, bash, shell, shell-session
- json, yaml, yml, toml
- sql, html, css

**Not highlighted:**
- Missing language tags
- `text` language
- Unknown/unapproved languages

This ensures:
1. **Performance**: No arbitrary language parsing
2. **Security**: Controlled syntax highlighting surface
3. **Consistency**: Same input → same output

## Runtime Surface Styles

All code/terminal/process surfaces use centralized CSS variables:

```css
--chat-runtime-surface-bg
--chat-runtime-surface-border
--chat-runtime-surface-text
```

These adapt automatically to light/dark themes.

## Migration from ai-elements

The following components have been migrated here from `@/components/ai-elements`:

- `message.tsx` ← `ai-elements/message.tsx`
- `reasoning.tsx` ← `ai-elements/reasoning.tsx`
- `plan.tsx` ← `ai-elements/plan.tsx`
- `tool.tsx` ← `ai-elements/tool.tsx`
- `terminal.tsx` ← `ai-elements/terminal.tsx`
- `code-block.tsx` ← `ai-elements/code-block.tsx`

Old imports are deprecated but still work via re-exports.
