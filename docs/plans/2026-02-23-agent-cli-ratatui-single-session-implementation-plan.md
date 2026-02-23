# Agent CLI (ratatui) Single-Session Multi-Turn Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a new `agent-cli` binary that launches directly into a ratatui chat UI, defaults to creating a new session, and supports resuming history with `--session <session_id>`.

**Architecture:** Add a dedicated `agent-cli` crate and keep existing `agent-turn-cli`/`agent-session-cli` unchanged. Split UI state/reducer, stream runtime adapter, rendering, and event loop into separate modules so non-terminal logic is testable without TTY. Reuse `agent` facade as the only runtime/session backend.

**Tech Stack:** Rust, tokio, clap, ratatui, crossterm, futures, agent facade (`agent`, `agent-core`, `agent-turn`, `bigmodel-api`)

---

**Skill references:** `@domain-cli`, `@test-driven-development`, `@rust-router`, `@verification-before-completion`

### Task 1: Workspace + Crate Scaffold (TDD)

**Files:**
- Modify: `Cargo.toml`
- Create: `agent-cli/Cargo.toml`
- Create: `agent-cli/src/main.rs`
- Create: `agent-cli/src/lib.rs`
- Test: `agent-cli/tests/cli_help.rs`

**Step 1: Write the failing test**

```rust
// agent-cli/tests/cli_help.rs
use std::process::Command;

#[test]
fn help_shows_chat_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-cli"))
        .arg("--help")
        .output()
        .expect("run agent-cli --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--session"));
    assert!(stdout.contains("--store-dir"));
    assert!(stdout.contains("--api-key"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli --test cli_help -q`  
Expected: FAIL (package `agent-cli` not found or missing binary)

**Step 3: Write minimal implementation**

```toml
# Cargo.toml (workspace root) - add member
members = [
  # ...existing
  "agent-cli"
]
```

```toml
# agent-cli/Cargo.toml
[package]
name = "agent-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
agent = { path = "../agent" }
agent-core = { path = "../agent-core" }
agent-turn = { path = "../agent-turn" }
bigmodel-api = { path = "../bigmodel-api" }
anyhow.workspace = true
clap.workspace = true
futures.workspace = true
tokio.workspace = true
ratatui = "0.29"
crossterm = "0.28"

[dev-dependencies]
tempfile.workspace = true
```

```rust
// agent-cli/src/main.rs
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "agent-cli")]
struct Cli {
    #[arg(long, env = "BIGMODEL_API_KEY")]
    api_key: String,
    #[arg(long)]
    session: Option<String>,
    #[arg(long)]
    store_dir: Option<std::path::PathBuf>,
}

fn main() {
    let _ = Cli::parse();
}
```

```rust
// agent-cli/src/lib.rs
pub fn crate_ready() -> bool {
    true
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli --test cli_help -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml agent-cli/Cargo.toml agent-cli/src/main.rs agent-cli/src/lib.rs agent-cli/tests/cli_help.rs
git commit -m "feat(agent-cli): scaffold crate and baseline CLI args"
```

### Task 2: CLI Config Module and Parse Tests

**Files:**
- Modify: `agent-cli/src/main.rs`
- Modify: `agent-cli/src/lib.rs`
- Create: `agent-cli/src/cli.rs`
- Test: `agent-cli/src/cli.rs`

**Step 1: Write the failing test**

```rust
// inside agent-cli/src/cli.rs #[cfg(test)]
#[test]
fn parse_defaults_to_new_session_mode() {
    let args = ["agent-cli", "--api-key", "k"];
    let cfg = CliArgs::parse_from(args);
    assert!(cfg.session.is_none());
}

#[test]
fn parse_accepts_session_resume() {
    let args = ["agent-cli", "--api-key", "k", "--session", "s-1"];
    let cfg = CliArgs::parse_from(args);
    assert_eq!(cfg.session.as_deref(), Some("s-1"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli cli::tests::parse_defaults_to_new_session_mode -q`  
Expected: FAIL (`CliArgs` not found)

**Step 3: Write minimal implementation**

```rust
// agent-cli/src/cli.rs
use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "agent-cli", about = "Terminal chat UI for agent facade")]
pub struct CliArgs {
    #[arg(long, env = "BIGMODEL_API_KEY")]
    pub api_key: String,
    #[arg(long, env = "BIGMODEL_BASE_URL", default_value = "https://open.bigmodel.cn/api/paas/v4")]
    pub base_url: String,
    #[arg(long, default_value = "glm-4.5")]
    pub model: String,
    #[arg(long)]
    pub system_prompt: Option<String>,
    #[arg(long)]
    pub max_tokens: Option<i32>,
    #[arg(long)]
    pub temperature: Option<f32>,
    #[arg(long)]
    pub top_p: Option<f32>,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub store_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub debug_events: bool,
}
```

```rust
// agent-cli/src/lib.rs
pub mod cli;
```

```rust
// agent-cli/src/main.rs
use clap::Parser;
use agent_cli::cli::CliArgs;

fn main() {
    let _ = CliArgs::parse();
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli cli::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/main.rs agent-cli/src/lib.rs agent-cli/src/cli.rs
git commit -m "feat(agent-cli): extract and test CLI argument parsing"
```

### Task 3: Session Bootstrap Policy (`--session` resume vs auto-create)

**Files:**
- Create: `agent-cli/src/session.rs`
- Modify: `agent-cli/src/lib.rs`
- Test: `agent-cli/src/session.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn no_session_arg_creates_new_session() {
    let gateway = FakeGateway::default();
    let id = resolve_session_id(&gateway, None).await.unwrap();
    assert_eq!(id, "new-session");
}

#[tokio::test]
async fn provided_session_must_exist() {
    let gateway = FakeGateway::with_existing(["s-1"]);
    let id = resolve_session_id(&gateway, Some("s-1")).await.unwrap();
    assert_eq!(id, "s-1");
}

#[tokio::test]
async fn missing_provided_session_returns_error() {
    let gateway = FakeGateway::default();
    let err = resolve_session_id(&gateway, Some("missing")).await.unwrap_err();
    assert!(err.to_string().contains("session not found"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli session::tests::no_session_arg_creates_new_session -q`  
Expected: FAIL (`resolve_session_id` not found)

**Step 3: Write minimal implementation**

```rust
use anyhow::{anyhow, Result};
use async_trait::async_trait;

#[async_trait]
pub trait SessionGateway: Send + Sync {
    async fn create_session(&self) -> Result<String>;
    async fn session_exists(&self, session_id: &str) -> Result<bool>;
}

pub async fn resolve_session_id<G: SessionGateway>(
    gateway: &G,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(session_id) = requested {
        if gateway.session_exists(session_id).await? {
            return Ok(session_id.to_string());
        }
        return Err(anyhow!("session not found: {session_id}"));
    }

    gateway.create_session().await
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli session::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/session.rs agent-cli/src/lib.rs
git commit -m "feat(agent-cli): add and test session bootstrap policy"
```

### Task 4: App State Reducer for Multi-Turn UI

**Files:**
- Create: `agent-cli/src/app.rs`
- Modify: `agent-cli/src/lib.rs`
- Test: `agent-cli/src/app.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn submit_appends_user_message_and_locks_turn() {
    let mut app = AppState::new("s-1".to_string());
    app.input = "hello".to_string();
    let cmd = app.submit_input().unwrap();

    assert_eq!(cmd.session_id, "s-1");
    assert_eq!(cmd.message, "hello");
    assert!(app.active_turn.is_some());
    assert_eq!(app.messages.last().unwrap().role, Role::User);
}

#[test]
fn cannot_submit_while_turn_is_active() {
    let mut app = AppState::new("s-1".to_string());
    app.active_turn = Some(ActiveTurn::default());
    app.input = "second".to_string();

    let cmd = app.submit_input();
    assert!(cmd.is_none());
    assert!(app.last_warning.as_deref() == Some("turn in progress"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli app::tests::submit_appends_user_message_and_locks_turn -q`  
Expected: FAIL (`AppState` not found)

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageItem {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveTurn {
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitCommand {
    pub session_id: String,
    pub message: String,
}

pub struct AppState {
    pub session_id: String,
    pub input: String,
    pub messages: Vec<MessageItem>,
    pub active_turn: Option<ActiveTurn>,
    pub last_warning: Option<String>,
    pub show_reasoning: bool,
}

impl AppState {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            input: String::new(),
            messages: Vec::new(),
            active_turn: None,
            last_warning: None,
            show_reasoning: true,
        }
    }

    pub fn submit_input(&mut self) -> Option<SubmitCommand> {
        if self.active_turn.is_some() {
            self.last_warning = Some("turn in progress".to_string());
            return None;
        }

        let message = self.input.trim().to_string();
        if message.is_empty() {
            return None;
        }

        self.messages.push(MessageItem {
            role: Role::User,
            text: message.clone(),
        });
        self.input.clear();
        self.active_turn = Some(ActiveTurn::default());
        Some(SubmitCommand {
            session_id: self.session_id.clone(),
            message,
        })
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli app::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/app.rs agent-cli/src/lib.rs
git commit -m "feat(agent-cli): add test-driven app state reducer"
```

### Task 5: Stream Event Mapping (assistant/reasoning/tool progress)

**Files:**
- Create: `agent-cli/src/runtime.rs`
- Modify: `agent-cli/src/app.rs`
- Modify: `agent-cli/src/lib.rs`
- Test: `agent-cli/src/runtime.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn maps_ui_and_run_events_to_app_events() {
    let ev = map_stream_event(AgentStreamEvent::Ui(UiThreadEvent::ReasoningDelta {
        turn_id: "t1".into(),
        delta: "thinking".into(),
    }));
    assert!(matches!(ev, Some(AppEvent::ReasoningDelta { .. })));

    let done = map_stream_event(AgentStreamEvent::Run(RunStreamEvent::TurnDone {
        turn_id: "t1".into(),
        epoch: 0,
        final_message: None,
        usage: agent_core::Usage::default(),
        stats: agent_core::TurnStats::default(),
    }));
    assert!(matches!(done, Some(AppEvent::TurnFinished { .. })));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli runtime::tests::maps_ui_and_run_events_to_app_events -q`  
Expected: FAIL (`map_stream_event` not found)

**Step 3: Write minimal implementation**

```rust
use agent::{AgentStreamEvent};
use agent_core::{RunStreamEvent, UiThreadEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    AssistantDelta { delta: String },
    ReasoningDelta { delta: String },
    ToolRequested { call_id: String, tool_name: String },
    ToolProgress { call_id: String, status: String },
    ToolCompleted { call_id: String },
    Warning { message: String },
    Error { message: String },
    TurnFinished { turn_id: String, failed: bool },
}

pub fn map_stream_event(event: AgentStreamEvent) -> Option<AppEvent> {
    match event {
        AgentStreamEvent::Ui(UiThreadEvent::MessageDelta { delta, .. }) => {
            Some(AppEvent::AssistantDelta { delta })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ReasoningDelta { delta, .. }) => {
            Some(AppEvent::ReasoningDelta { delta })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallRequested { call_id, tool_name, .. }) => {
            Some(AppEvent::ToolRequested { call_id, tool_name })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallProgress { call_id, status, .. }) => {
            Some(AppEvent::ToolProgress {
                call_id,
                status: format!("{status:?}"),
            })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallCompleted { result, .. }) => {
            Some(AppEvent::ToolCompleted {
                call_id: result.call_id,
            })
        }
        AgentStreamEvent::Ui(UiThreadEvent::Warning { message, .. }) => {
            Some(AppEvent::Warning { message })
        }
        AgentStreamEvent::Ui(UiThreadEvent::Error { message, .. }) => {
            Some(AppEvent::Error { message })
        }
        AgentStreamEvent::Run(RunStreamEvent::TurnDone { turn_id, .. }) => {
            Some(AppEvent::TurnFinished { turn_id, failed: false })
        }
        AgentStreamEvent::Run(RunStreamEvent::TurnFailed { turn_id, .. }) => {
            Some(AppEvent::TurnFinished { turn_id, failed: true })
        }
        _ => None,
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli runtime::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/runtime.rs agent-cli/src/app.rs agent-cli/src/lib.rs
git commit -m "feat(agent-cli): add stream-to-app event mapping"
```

### Task 6: ratatui Rendering (history + input + tool status + reasoning fold)

**Files:**
- Create: `agent-cli/src/ui.rs`
- Modify: `agent-cli/src/app.rs`
- Modify: `agent-cli/src/lib.rs`
- Test: `agent-cli/src/ui.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn render_shows_input_and_messages() {
    use ratatui::{backend::TestBackend, Terminal};

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = AppState::new("s-1".into());
    app.input = "hello".into();
    app.messages.push(MessageItem { role: Role::Assistant, text: "hi".into() });

    terminal.draw(|frame| draw(frame, &app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let full = format!("{}", buf);
    assert!(full.contains("hi"));
    assert!(full.contains("hello"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli ui::tests::render_shows_input_and_messages -q`  
Expected: FAIL (`draw` not found)

**Step 3: Write minimal implementation**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;

pub fn draw(frame: &mut Frame<'_>, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let history_text = app
        .messages
        .iter()
        .map(|m| m.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let history = Paragraph::new(history_text)
        .block(Block::default().title("Chat").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(history, chunks[0]);

    let input = Paragraph::new(app.input.clone())
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli ui::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/ui.rs agent-cli/src/app.rs agent-cli/src/lib.rs
git commit -m "feat(agent-cli): add baseline ratatui rendering with tests"
```

### Task 7: Keyboard Controller + Event Loop Decisions

**Files:**
- Create: `agent-cli/src/event_loop.rs`
- Modify: `agent-cli/src/app.rs`
- Modify: `agent-cli/src/lib.rs`
- Test: `agent-cli/src/event_loop.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn esc_requests_exit() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = AppState::new("s-1".into());
    let action = handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert!(matches!(action, LoopAction::Quit));
}

#[test]
fn tab_toggles_reasoning_visibility() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = AppState::new("s-1".into());
    assert!(app.show_reasoning);
    let _ = handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert!(!app.show_reasoning);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli event_loop::tests -q`  
Expected: FAIL (`handle_key_event` not found)

**Step 3: Write minimal implementation**

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    None,
    Submit,
    Quit,
}

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> LoopAction {
    match key.code {
        KeyCode::Esc => LoopAction::Quit,
        KeyCode::Tab => {
            app.show_reasoning = !app.show_reasoning;
            LoopAction::None
        }
        KeyCode::Enter => LoopAction::Submit,
        KeyCode::Char(c) => {
            app.input.push(c);
            LoopAction::None
        }
        KeyCode::Backspace => {
            app.input.pop();
            LoopAction::None
        }
        _ => LoopAction::None,
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-cli event_loop::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/event_loop.rs agent-cli/src/app.rs agent-cli/src/lib.rs
git commit -m "feat(agent-cli): add keyboard controller and loop actions"
```

### Task 8: Agent Runtime Wiring (BigModel + session bootstrap + stream pump)

**Files:**
- Modify: `agent-cli/src/main.rs`
- Modify: `agent-cli/src/runtime.rs`
- Modify: `agent-cli/src/session.rs`
- Modify: `agent-cli/src/cli.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn stream_pump_emits_turn_finished() {
    use futures::stream;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let events = vec![
        agent::AgentStreamEvent::Ui(agent_core::UiThreadEvent::MessageDelta {
            turn_id: "t1".into(),
            delta: "hello".into(),
        }),
        agent::AgentStreamEvent::Run(agent_core::RunStreamEvent::TurnDone {
            turn_id: "t1".into(),
            epoch: 0,
            final_message: Some("hello".into()),
            usage: agent_core::Usage::default(),
            stats: agent_core::TurnStats::default(),
        }),
    ];
    let stream: agent::AgentStream = Box::pin(stream::iter(events));

    pump_stream(stream, tx).await;

    let mut got_finish = false;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, AppEvent::TurnFinished { failed: false, .. }) {
            got_finish = true;
        }
    }
    assert!(got_finish);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-cli runtime::tests::stream_pump_emits_turn_finished -q`  
Expected: FAIL (`pump_stream` not found)

**Step 3: Write minimal implementation**

```rust
// runtime.rs
pub async fn pump_stream(
    mut stream: agent::AgentStream,
    tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
) {
    use futures::StreamExt;
    while let Some(event) = stream.next().await {
        if let Some(mapped) = map_stream_event(event) {
            if tx.send(mapped).is_err() {
                break;
            }
        }
    }
}
```

```rust
// main.rs (real wiring skeleton)
let args = CliArgs::parse();
let client = std::sync::Arc::new(bigmodel_api::BigModelClient::new(
    bigmodel_api::Config::new(args.api_key.clone()).with_base_url(args.base_url.clone()),
));
let model_cfg = agent_turn::adapters::bigmodel::BigModelAdapterConfig {
    model: args.model.clone(),
    system_prompt: args.system_prompt.clone(),
    max_tokens: args.max_tokens,
    temperature: args.temperature,
    top_p: args.top_p,
};
let model = std::sync::Arc::new(
    agent_turn::adapters::bigmodel::BigModelModelAdapter::new(client).with_config(model_cfg),
);

let mut builder = agent::AgentBuilder::new().model(model);
if let Some(store_dir) = args.store_dir.clone() {
    builder = builder.store_dir(store_dir);
}
let agent = builder.build().await?;

let gateway = AgentSessionGateway::new(&agent);
let session_id = resolve_session_id(&gateway, args.session.as_deref()).await?;
let mut app = AppState::new(session_id);
run_tui_loop(&agent, &mut app, args.debug_events).await?;
```

**Step 4: Run targeted tests to verify pass**

Run: `cargo test -p agent-cli runtime::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/main.rs agent-cli/src/runtime.rs agent-cli/src/session.rs agent-cli/src/cli.rs
git commit -m "feat(agent-cli): wire agent runtime bootstrap and stream pumping"
```

### Task 9: Multi-Turn and `--session` Resume Integration Tests

**Files:**
- Create: `agent-cli/tests/session_policy_integration.rs`
- Create: `agent-cli/tests/multi_turn_integration.rs`
- Modify: `agent-cli/src/runtime.rs` (if test seam needed)

**Step 1: Write the failing tests**

```rust
use std::sync::Arc;
use async_trait::async_trait;
use futures::stream;
use tempfile::TempDir;

struct EchoModel;

#[async_trait]
impl agent_core::LanguageModel for EchoModel {
    fn model_name(&self) -> &str { "echo" }

    async fn stream(
        &self,
        request: agent_core::ModelRequest,
    ) -> Result<agent_core::ModelEventStream, agent_core::AgentError> {
        let delta = format!("history={}", request.transcript.len());
        Ok(Box::pin(stream::iter(vec![
            Ok(agent_core::ModelOutputEvent::TextDelta { delta }),
            Ok(agent_core::ModelOutputEvent::Completed { usage: None }),
        ])))
    }
}

#[tokio::test]
async fn same_session_supports_two_turns() {
    let temp = TempDir::new().unwrap();
    let agent = agent::AgentBuilder::new()
        .model(Arc::new(EchoModel))
        .store_dir(temp.path().to_path_buf())
        .build()
        .await
        .unwrap();

    let session_id = agent.create_session(None, Some("demo".into())).await.unwrap();
    let first = agent.chat(&session_id, "first").await.unwrap();
    let second = agent.chat(&session_id, "second").await.unwrap();

    assert_eq!(first.final_message.as_deref(), Some("history=1"));
    assert_eq!(second.final_message.as_deref(), Some("history=2"));
}

#[tokio::test]
async fn resume_mode_rejects_missing_session() {
    #[derive(Default)]
    struct FakeGateway;
    #[async_trait]
    impl SessionGateway for FakeGateway {
        async fn create_session(&self) -> anyhow::Result<String> { Ok("new".into()) }
        async fn session_exists(&self, _session_id: &str) -> anyhow::Result<bool> { Ok(false) }
    }

    let err = resolve_session_id(&FakeGateway, Some("missing")).await.unwrap_err();
    assert!(err.to_string().contains("session not found: missing"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-cli --test multi_turn_integration -q`  
Expected: FAIL (test helper types or imports missing)

**Step 3: Write minimal implementation to satisfy tests**

```rust
// session.rs
pub struct AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    agent: &'a agent::Agent<L>,
}

impl<'a, L> AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub fn new(agent: &'a agent::Agent<L>) -> Self { Self { agent } }
}

#[async_trait::async_trait]
impl<'a, L> SessionGateway for AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    async fn create_session(&self) -> anyhow::Result<String> {
        Ok(self.agent.create_session(None, Some("agent-cli".into())).await?)
    }

    async fn session_exists(&self, session_id: &str) -> anyhow::Result<bool> {
        Ok(self.agent.get_session(session_id).await?.is_some())
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-cli --test session_policy_integration --test multi_turn_integration -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/tests/session_policy_integration.rs agent-cli/tests/multi_turn_integration.rs agent-cli/src/runtime.rs
git commit -m "test(agent-cli): cover multi-turn flow and session resume policy"
```

### Task 10: Terminal UX Polish and Regression Guard

**Files:**
- Modify: `agent-cli/src/ui.rs`
- Modify: `agent-cli/src/app.rs`
- Modify: `agent-cli/src/event_loop.rs`
- Test: `agent-cli/src/ui.rs`

**Step 1: Write failing tests for reasoning/tool visibility**

```rust
#[test]
fn folded_reasoning_hides_reasoning_lines() {
    use ratatui::{backend::TestBackend, Terminal};
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = AppState::new("s-1".into());
    app.show_reasoning = false;
    app.reasoning_text = "secret reasoning".into();

    terminal.draw(|frame| draw(frame, &app)).unwrap();
    let full = format!("{}", terminal.backend().buffer());
    assert!(!full.contains("secret reasoning"));
}

#[test]
fn tool_progress_renders_status_labels() {
    use ratatui::{backend::TestBackend, Terminal};
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = AppState::new("s-1".into());
    app.tool_progress.push(ToolProgressItem {
        tool_name: "read_file".into(),
        status: "running".into(),
    });

    terminal.draw(|frame| draw(frame, &app)).unwrap();
    let full = format!("{}", terminal.backend().buffer());
    assert!(full.contains("read_file"));
    assert!(full.contains("running"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-cli ui::tests::folded_reasoning_hides_reasoning_lines -q`  
Expected: FAIL

**Step 3: Implement minimal rendering updates**

```rust
// app.rs additions
pub struct ToolProgressItem {
    pub tool_name: String,
    pub status: String,
}

pub struct AppState {
    // ...existing
    pub reasoning_text: String,
    pub tool_progress: Vec<ToolProgressItem>,
}

// ui.rs additions
let mut history_lines = app.messages.iter().map(|m| m.text.clone()).collect::<Vec<_>>();
if app.show_reasoning && !app.reasoning_text.is_empty() {
    history_lines.push(format!("[reasoning] {}", app.reasoning_text));
}
for item in &app.tool_progress {
    history_lines.push(format!("[tool:{}] {}", item.tool_name, item.status));
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-cli ui::tests -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-cli/src/ui.rs agent-cli/src/app.rs agent-cli/src/event_loop.rs
git commit -m "feat(agent-cli): add reasoning fold and tool progress rendering"
```

### Task 11: Docs + Full Verification + Final Commit

**Files:**
- Modify: `docs/plans/2026-02-23-agent-cli-ratatui-single-session-design.md` (if implementation deviations exist)
- Create: `agent-cli/README.md`
- Modify: `agent-cli/src/main.rs` (help text polish if needed)

**Step 1: Add usage docs (failing doc check not required)**

```md
# agent-cli

## Usage
- New session: `cargo run -p agent-cli -- --api-key $BIGMODEL_API_KEY`
- Resume session: `cargo run -p agent-cli -- --api-key $BIGMODEL_API_KEY --session <session_id>`

## Keys
- Enter: send
- Tab: toggle reasoning
- Esc/Ctrl+C: quit
```

**Step 2: Run verification suite**

Run: `cargo fmt --all -- --check`  
Expected: PASS

Run: `cargo clippy -p agent-cli --all-targets --all-features -- -D warnings`  
Expected: PASS

Run: `cargo test -p agent-cli`  
Expected: PASS

**Step 3: If any verification fails, fix minimally and re-run same command**

```rust
// apply only focused fixes; do not broaden scope
```

**Step 4: Final commit**

```bash
git add agent-cli Cargo.toml docs/plans/2026-02-23-agent-cli-ratatui-single-session-design.md
git commit -m "feat(agent-cli): implement ratatui single-session multi-turn chat"
```

**Step 5: Optional integration sanity with existing tools**

Run: `cargo test -p agent-turn-cli -q && cargo test -p agent-session-cli -q`  
Expected: PASS (no regressions)

---

## Notes for Execution

- Keep scope strict: single session only, no in-app session switcher.
- Prefer small commits after each task.
- Do not refactor unrelated crates.
- Any extra UX ideas should be logged as follow-up TODOs, not implemented in this scope.
