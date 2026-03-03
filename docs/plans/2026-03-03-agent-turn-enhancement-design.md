# Agent-Turn Enhancement Design

**Date:** 2026-03-03
**Status:** Approved
**Author:** Claude (with user requirements)

## Overview

This design enhances the agent-turn system with comprehensive error handling, automatic retry with checkpoint recovery, and improved user feedback. The system will auto-retry up to 5 times for transient errors, use smart backoff strategies, and provide clear visual feedback throughout the process.

## Summary of Requirements

1. **Visual Feedback**: Use existing `Shimmer` component to show "Agent is analyzing..." when LLM is processing
2. **Retry Counter**: Display "Retrying... (N/5)" alongside shimmer during retry attempts
3. **Smart Retry**: Auto-retry transient errors (timeout, network) immediately, rate-limit with exponential backoff
4. **Permanent Errors**: No retry for auth failures, content policy violations, invalid requests
5. **Graceful Degradation**: After 5 failed retries, mark turn as "failed" with manual retry option
6. **Timeout Supervision**: Multi-level timeout protection (10min turn, 5min tool, 120s LLM)

## Architecture

### Error Categorization Hierarchy

```
Transient (Retry)
├─ Timeout (immediate retry)
├─ Network (immediate retry)
└─ RateLimit (exponential backoff)

Permanent (No Retry)
├─ AuthenticationFailure
├─ ContentPolicyViolation
└─ InvalidRequest

Timeout Supervision
├─ StreamSupervisor (10min per turn)
├─ ToolTimeout (5min per tool)
└─ LLMTimeout (existing 120s request)
```

### Component Layers

```
┌─────────────────────────────────────────┐
│              Frontend                   │
│  Shimmer | RetryCounter | ErrorBanner   │
└─────────────────────────────────────────┘
                  ↕ agent:stream
┌─────────────────────────────────────────┐
│           Tauri Backend                │
│  RetryOrchestrator | TurnSupervisor    │
└─────────────────────────────────────────┘
                  ↕
┌─────────────────────────────────────────┐
│          Agent Runtime                  │
│  Stream Forwarder | Tool Executor       │
└─────────────────────────────────────────┘
```

## Backend Modifications

### New Components

**A. RetryOrchestrator** (`desktop/src-tauri/src/retry_orchestrator.rs` - new file)

```rust
struct RetryState {
    turn_id: String,
    attempt: u8,
    max_attempts: u8,
    last_checkpoint: Option<String>,
    error_history: Vec<String>,
}

pub struct RetryConfig {
    pub max_attempts: u8,
    pub immediate_retry: bool,
}

impl RetryOrchestrator {
    pub fn backoff_delay(&self, attempt: u8, requires_backoff: bool) -> Duration {
        if !requires_backoff {
            return Duration::ZERO;
        }
        let secs = 2u64.pow(attempt as u32).min(30);
        Duration::from_secs(secs)
    }

    pub fn should_retry(&self, attempt: u8) -> bool {
        attempt < self.config.max_attempts
    }
}
```

**B. TurnSupervisor** (`desktop/src-tauri/src/lib.rs`)

```rust
struct TurnSupervisor {
    turn_id: String,
    timeout: Duration,
    tx: mpsc::Sender<SupervisorEvent>,
}

#[tauri::command]
async fn retry_failed_turn(
    state: State<'_, AppState>,
    session_id: String,
    turn_id: String,
) -> Result<StartAgentTurnResponse, String>
```

**C. Tool Execution Timeout** (`agent-turn/src/effect.rs`)

```rust
async fn execute_tool_with_timeout(
    tools: Arc<AgentToolRuntime>,
    call: ToolCall,
    ctx: ToolExecutionContext,
    timeout: Duration,
) -> ToolResult {
    tokio::time::timeout(timeout, tools.execute_tool(call, ctx)).await
}
```

**D. New Runtime Events** (`agent-core/src/runtime.rs`)

```rust
pub enum RuntimeEvent {
    Retrying {
        event_id: String,
        epoch: u64,
        attempt: u8,
        max_attempts: u8,
        error: String,
    },
    ToolTimeout {
        event_id: String,
        epoch: u64,
        call_id: String,
        tool_name: String,
        duration_ms: u64,
    },
    SupervisionTimeout {
        event_id: String,
        turn_id: String,
        reason: String,
    },
}
```

**E. Error Classification** (`llm-client/src/error.rs`)

```rust
impl LlmError {
    pub fn is_transient(&self) -> bool {
        matches!(self,
            LlmError::Timeout { .. }
            | LlmError::NetworkError { .. }
            | LlmError::StreamIdleTimeout { .. }
            | LlmError::RequestTimeout
            | LlmError::ServerError { status: 429..=599, .. }
        )
    }

    pub fn requires_backoff(&self) -> bool {
        matches!(self,
            LlmError::ServerError { status: 429, .. }
            | LlmError::ServerError { status: 503, .. }
        )
    }

    pub fn is_permanent(&self) -> bool {
        matches!(self,
            LlmError::AuthenticationFailed { .. }
            | LlmError::ContentPolicyViolation { .. }
            | LlmError::InvalidRequest { .. }
            | LlmError::ServerError { status: 400..=404, .. }
        )
    }

    pub fn user_message(&self) -> String { /* ... */ }
}
```

## Frontend Components

### New Components

**A. ThinkingIndicator** (`desktop/components/features/chat/thinking-indicator.tsx` - new file)

```tsx
export function ThinkingIndicator({ retryState }: { retryState?: RetryState }) {
  return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm">
      <Shimmer>Agent is analyzing...</Shimmer>
      {retryState && (
        <span className="text-xs">
          Retrying... ({retryState.attempt}/{retryState.maxAttempts})
        </span>
      )}
    </div>
  );
}
```

**B. TurnErrorBanner** (`desktop/components/features/chat/turn-error-banner.tsx` - new file)

```tsx
export function TurnErrorBanner({ error, onRetry }: {
  error: string;
  onRetry: () => void;
}) {
  return (
    <Alert variant="destructive">
      <AlertCircle className="h-4 w-4" />
      <AlertDescription className="flex items-center justify-between">
        <span>{error}</span>
        <Button size="sm" variant="outline" onClick={onRetry}>
          Retry from checkpoint
        </Button>
      </AlertDescription>
    </Alert>
  );
}
```

**C. AgentTurnCard Enhancement** (`desktop/components/features/chat/agent-turn-card.tsx`)

```tsx
type RetryState = {
  isRetrying: boolean;
  attempt: number;
  maxAttempts: number;
  lastError?: string;
}

interface AgentTurnVM {
  // ... existing fields
  retryState?: RetryState;
  failedPermanently?: boolean;
  permanentError?: string;
}
```

**D. Store Enhancement** (`desktop/lib/stores/chat-store.ts`)

```typescript
applyAgentStreamEnvelope: (envelope: AgentStreamEnvelope) => {
  switch (event.source) {
    case 'Retrying':
      updateTurn(turnId, { retryState: { ... } });
      break;
    case 'FatalError':
    case 'SupervisionTimeout':
      updateTurn(turnId, { status: 'failed', failedPermanently: true });
      break;
  }
}

retryTurnFromCheckpoint: async (sessionId: string, turnId: string) => {
  const result = await retryFailedTurn({ sessionId, turnId });
  return result;
}
```

## Data Flow

### Normal Flow (No Error)

```
User sends message
    ↓
Frontend: Show Shimmer "Agent is analyzing..."
    ↓
Tauri: start_agent_turn() → return turn_id
    ↓
Backend: Start TurnSupervisor (10min timeout)
    ↓
Runtime: Execute turn → emit RuntimeEvent
    ↓
Frontend: Receive stream events → update UI
    ↓
Turn complete: status = "done" → Hide Shimmer, show TurnCheckpoint
```

### Error + Auto-Retry Flow

```
Error during turn execution
    ↓
Runtime: Detect LlmError → is_transient() = true
    ↓
RetryOrchestrator: attempt < 5?
    Yes: Save checkpoint → Send Retrying event
    ↓
Frontend: Show "Retrying... (1/5)", Shimmer continues
    ↓
Backend: Restore from checkpoint, re-execute
    ↓
[Success] → Complete turn
[Fail] → Repeat retry flow
```

### Permanent Failure Flow

```
Error during turn execution
    ↓
Runtime: Detect LlmError → is_permanent() = true
    ↓
Backend: Send FatalError event, mark status = "failed"
    ↓
Frontend: Hide Shimmer/RetryCounter, show TurnErrorBanner
```

### Timeout Flows

**TurnSupervisor Timeout (10min):**
```
Supervisor detects timeout → Send SupervisionTimeout → Cancel all tasks → Trigger retry or fatal error
```

**Tool Timeout (5min):**
```
ToolExecutor → tokio::time::timeout → Send ToolTimeout → Treat as transient → Trigger retry
```

## Testing Strategy

### Unit Tests

- Error classification: `is_transient()`, `is_permanent()`, `requires_backoff()`
- Backoff calculation: exponential growth, cap at 30s
- Retry limit: `should_retry()` respects max_attempts

### Integration Tests

- Transient error triggers retry flow
- Permanent error bypasses retry
- Supervisor timeout cancels and reports
- Tool timeout triggers retry

### Frontend Tests

- `ThinkingIndicator` shows shimmer and retry counter
- `TurnErrorBanner` displays error with retry button
- `chat-store` updates turn state on events

### Manual Test Scenarios

| Scenario | Expected Result |
|----------|-----------------|
| Normal flow | Shimmer → Complete → Checkpoint |
| Network timeout | "Retrying... (1/5)" → Success |
| Rate Limit (429) | "Retrying... (1/5)" with backoff → Success |
| Auth failure (401) | Error banner, no retry |
| Tool timeout (>5min) | "Retrying..." → Success or fail |
| Supervisor timeout (>10min) | "Retrying..." or error banner |
| Max retries (5x) | Error banner + manual retry button |

## Files to Modify

### Backend (Rust)
- `llm-client/src/error.rs` - Error classification methods
- `agent-turn/src/effect.rs` - Tool execution timeout
- `agent-core/src/runtime.rs` - New RuntimeEvent types
- `desktop/src-tauri/src/lib.rs` - RetryOrchestrator, TurnSupervisor
- `desktop/src-tauri/src/retry_orchestrator.rs` - **NEW FILE**

### Frontend (TypeScript/React)
- `desktop/components/features/chat/thinking-indicator.tsx` - **NEW FILE**
- `desktop/components/features/chat/turn-error-banner.tsx` - **NEW FILE**
- `desktop/components/features/chat/agent-turn-card.tsx` - Enhance
- `desktop/lib/stores/chat-store.ts` - Retry state handling
- `desktop/lib/api/chat.ts` - retry_failed_turn API

## Implementation Priority

1. **Phase 1**: Error classification + backend retry orchestration
2. **Phase 2**: Timeout supervision (TurnSupervisor, tool timeout)
3. **Phase 3**: Frontend components (ThinkingIndicator, ErrorBanner)
4. **Phase 4**: Store integration + stream event handling
5. **Phase 5**: Testing (unit, integration, manual)

## Success Criteria

- ✅ Users see "Agent is analyzing..." during turn execution
- ✅ Transient errors auto-retry up to 5 times
- ✅ Retry counter shows progress "Retrying... (N/5)"
- ✅ Permanent errors show user-friendly message immediately
- ✅ No orphaned tasks (TurnSupervisor enforces 10min limit)
- ✅ Tool execution respects 5min timeout
- ✅ Checkpoint recovery works on retry
- ✅ Conversation state preserved on failure
