---
name: ai-sdk-elements-fetcher
description: Use when fetching source code, examples, or documentation from Vercel AI SDK Elements website (elements.ai-sdk.dev) or when asked to get component code from AI Elements. Keywords: fetch source code, component examples, Message, Reasoning, Attachments, Conversation, Code Block, Chain of Thought
---

# AI SDK Elements Fetcher

## Overview
Quickly retrieve component source code and examples from the Vercel AI SDK Elements documentation site using direct URL patterns.

## Component List

### Chatbot Components
- Attachments
- Chain of Thought
- Checkpoint
- Confirmation
- Context
- Conversation
- Inline Citation
- Message
- Model Selector
- Plan
- Prompt Input
- Queue
- Reasoning
- Shimmer
- Sources
- Suggestion
- Task
- Tool

### Code Components
- Agent
- Artifact
- Code Block
- Commit
- Environment Variables
- File Tree
- JSX Preview
- Package Info
- Sandbox
- Schema Display
- Snippet
- Stack Trace
- Terminal
- Test Results
- Web Preview

### Voice Components
- Audio Player
- Mic Selector
- Persona
- Speech Input
- Transcription
- Voice Selector

### Workflow Components
- Canvas
- Connection
- Controls
- Edge
- Node
- Panel
- Toolbar

## Quick Reference

| Task | URL Pattern |
|------|-------------|
| Component documentation with code | `https://elements.ai-sdk.dev/components/{component-name}` |
| Raw markdown with code blocks | `https://elements.ai-sdk.dev/components/{component-name}.md` |

**Component names use lowercase with hyphens:** `chain-of-thought`, `model-selector`, `jsx-preview`

## Implementation

Use the web-reader tool to fetch component pages:

```typescript
// For a specific component
web-reader("https://elements.ai-sdk.dev/components/reasoning.md")

// For the components list
web-reader("https://elements.ai-sdk.dev/components")
```

**Response includes:**
- Full component source code in markdown code blocks
- Usage examples
- Props/API documentation
- TypeScript type definitions

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Guessing URLs like `/reasoning` or `/component/reasoning` | Use `/components/{name}` pattern |
| Fetching from GitHub repo | Use the `.md` URL - faster and includes examples |
| Manual navigation/search | Direct URL access - no search needed |
| Using web search | Direct URL pattern is known |

## Examples

Fetch Reasoning component code:
```
URL: https://elements.ai-sdk.dev/components/reasoning.md
```

Fetch Code Block component:
```
URL: https://elements.ai-sdk.dev/components/code-block.md
```

Fetch Chain of Thought component:
```
URL: https://elements.ai-sdk.dev/components/chain-of-thought.md
```
