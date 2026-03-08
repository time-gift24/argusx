# Agent Loop Design

## Overview

Implement a persistent Agent loop in Argusx that coordinates message handling, command parsing, and turn execution. The Agent runs continuously, listening for messages from multiple channels (Tauri IPC + extensible interface), processing system commands, and delegating turn execution to the existing TurnDriver.

## Goals

1. **Agent Persistence**: Agent runs as a long-lived loop, not dependent on external triggers
2. **Channel Abstraction**: Support Tauri IPC as primary channel, with extensible interface for future channels
3. **Command Parsing**: Full IronClaw-compatible command system (/undo, /resume, /compact, /clear, /new-thread, /switch-thread, /heartbeat, /summarize, /suggest)
4. **Hook System**: BeforeInbound/BeforeOutbound hooks for message interception
5. **Dual Mode**: Works both as desktop app (UI-driven) and standalone service (self-driven)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Agent                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Agent::run()                                          │   │
│  │    ├── subscribe_channel() → ChannelReceiver           │   │
│  │    ├── handle_message()                                │   │
│  │    │     ├── parse_command() → Command                │   │
│  │    │     ├── run_hook(BeforeInbound)                  │   │
│  │    │     └── dispatch() → Turn or CommandHandler      │   │
│  │    └── respond() → ChannelSender                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│  ┌───────────────────────────▼───────────────────────────┐   │
│  │  SessionManager                                        │   │
│  │    ├── create_thread(), send_message()                 │   │
│  │    └── turn_deps: Lazy<TurnDependencies>              │   │
│  └───────────────────────────────────────────────────────┘   │
│                              │                                   │
│  ┌───────────────────────────▼───────────────────────────┐   │
│  │  TurnDriver                                           │   │
│  │    └── Turn execution loop                             │   │
│  └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Channel Abstraction (`channel.rs`)

```rust
/// Message source abstraction
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn subscribe(&self) -> impl Stream<Item = ChannelMessage> + Send;
}

/// Outgoing response channel
pub trait ChannelSender: Send + Sync {
    async fn send(&self, user_id: &str, response: OutgoingResponse) -> Result<()>;
}
```

**Initial Implementation**:
- `TauriChannel`: Wraps Tauri IPC events, implements `Channel`
- `TauriSender`: Uses Tauri `invoke()` for responses

**Extensibility**:
- Define trait `Channel` + `ChannelSender`
- New channels (WebSocket, CLI, Webhook) implement these traits

### 2. Command System (`commands.rs`)

| Command | Description |
|---------|-------------|
| `/undo` | Undo last turn |
| `/resume [checkpoint_id]` | Resume from checkpoint |
| `/compact` | Compact context |
| `/clear` | Clear current thread |
| `/new-thread [title]` | Create new thread |
| `/switch-thread <id>` | Switch to thread |
| `/heartbeat` | Trigger heartbeat |
| `/summarize` | Summarize conversation |
| `/suggest` | Get suggestions |
| `/cancel` | Cancel current turn |

```rust
pub enum Command {
    Undo,
    Resume { checkpoint_id: Option<String> },
    Compact,
    Clear,
    NewThread { title: Option<String> },
    SwitchThread { thread_id: String },
    Heartbeat,
    Summarize,
    Suggest,
    Cancel,
    UserInput(String),  // Non-command text
}
```

### 3. Hook System (`hooks.rs`)

```rust
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    async fn before_inbound(&self, ctx: &HookContext) -> HookResult;
    async fn before_outbound(&self, ctx: &HookContext) -> HookResult;
}

pub enum HookEvent {
    Inbound { user_id, channel, content, thread_id },
    Outbound { user_id, channel, content, thread_id },
}

pub enum HookResult {
    Continue,
    ContinueWithModifications(String),
    Reject(String),  // Message rejected with reason
}
```

**Built-in Hooks**:
- `LoggingHook`: Logs all messages
- `FilteringHook`: Content filtering (future)

### 4. Agent Core (`agent.rs`)

```rust
pub struct Agent {
    session_manager: SessionManager,
    channel: Arc<dyn Channel>,
    sender: Arc<dyn ChannelSender>,
    hooks: Vec<Arc<dyn Hook>>,
    turn_deps: Lazy<TurnDependencies>,
}

impl Agent {
    pub async fn new(
        session_manager: SessionManager,
        channel: Arc<dyn Channel>,
        sender: Arc<dyn ChannelSender>,
    ) -> Self;

    pub async fn run(self) -> Result<()> {
        // 1. Subscribe to channel
        // 2. Initialize turn_deps on first use
        // 3. Main loop: receive → parse → hook → dispatch → respond
    }
}
```

### 5. TurnDependencies Lazy Initialization

```rust
pub struct TurnDependenciesBuilder {
    // Model, ToolRunner, Authorizer, Observer builders
}

impl TurnDependenciesBuilder {
    pub async fn build(&self) -> Result<TurnDependencies>;
}

// Lazy initialization in Agent
turn_deps: Lazy<TurnDependencies> = Lazy::new(|| async {
    // Initialize model, tools, authorizer, observer
    TurnDependenciesBuilder::new()
        .with_config(config)
        .build()
        .await
});
```

## Data Flow

```
1. Channel message arrives
       ↓
2. Agent::handle_message()
       ↓
3. CommandParser::parse() → Command or UserInput
       ↓
4. Hook: before_inbound(Command::UserInput)
       ↓ (if allowed)
5. Dispatch:
   - Command → CommandHandler
   - UserInput → SessionManager::send_message()
       ↓
6. Turn executes, events flow back via SessionEvent
       ↓
7. Hook: before_outbound(response)
       ↓
8. ChannelSender::send() → Frontend
```

## Error Handling

| Error Type | Handling |
|------------|----------|
| Channel disconnected | Log warning, attempt reconnect, continue loop |
| Turn execution error | Return error to channel, log |
| Hook rejection | Return rejection message to user |
| TurnDependencies init failure | Fatal: Agent exits with error |

## Testing Strategy

1. **Unit Tests**: CommandParser, Hook system
2. **Integration Tests**: Agent loop with mock channel
3. **E2E Tests**: Full flow with Tauri frontend

## File Structure

```
session/src/
├── agent.rs        # Agent main loop
├── channel.rs      # Channel trait + Tauri impl
├── commands.rs     # Command enum + parser
├── hooks.rs        # Hook trait + implementations
├── lazy.rs         # Lazy initialization helper
└── lib.rs         # Export all
```

## Breaking Changes

- `DesktopSessionState.turn_dependencies` changes from `Option<TurnDependencies>` to `Lazy<TurnDependencies>`
- `send_message` becomes internal; external interface is `Agent::run()`
- `SessionManager` adds `set_channel()`, `set_sender()`

## Success Criteria

1. Agent runs continuously without external message triggers
2. All IronClaw commands work identically
3. Hook system intercepts messages correctly
4. TurnDependencies initialized on first turn
5. Frontend receives all events via existing Tauri bridge
6. Can extend with new Channel implementations without Agent changes
