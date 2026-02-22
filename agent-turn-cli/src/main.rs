use std::io::{self, IsTerminal, Read, Write};
use std::sync::Arc;

use agent_core::{
    new_id, InputEnvelope, RunEventStream, RunStreamEvent, Runtime, SessionMeta, TurnRequest,
    UiEventStream, UiThreadEvent,
};
use agent_turn::adapters::bigmodel::{BigModelAdapterConfig, BigModelModelAdapter};
use agent_turn::effect::ToolExecutor;
use agent_turn::state::RetryPolicy;
use agent_turn::{TurnEngineConfig, TurnRuntime};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use bigmodel_api::{BigModelClient, Config};
use clap::Parser;
use futures::StreamExt;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "agent-turn-cli")]
#[command(about = "Manual test CLI for agent-turn runtime")]
struct Cli {
    #[arg(help = "Prompt text; if omitted, reads from stdin")]
    prompt: Option<String>,

    #[arg(long, env = "BIGMODEL_API_KEY")]
    api_key: String,

    #[arg(
        long,
        env = "BIGMODEL_BASE_URL",
        default_value = "https://open.bigmodel.cn/api/paas/v4"
    )]
    base_url: String,

    #[arg(long, default_value = "glm-4.5")]
    model: String,

    #[arg(long)]
    system_prompt: Option<String>,

    #[arg(long)]
    max_tokens: Option<i32>,

    #[arg(long)]
    temperature: Option<f32>,

    #[arg(long)]
    top_p: Option<f32>,

    #[arg(long)]
    session_id: Option<String>,

    #[arg(long)]
    turn_id: Option<String>,

    #[arg(long, help = "Print run stream events")]
    show_run: bool,

    #[arg(long, help = "Output all events in JSON lines")]
    json: bool,

    #[arg(long, default_value_t = 4)]
    max_parallel_tools: usize,

    #[arg(long, default_value_t = 3)]
    max_retries: u32,

    #[arg(long, default_value_t = 200)]
    base_delay_ms: u64,
}

struct CliToolExecutor;

#[async_trait]
impl ToolExecutor for CliToolExecutor {
    async fn execute_tool(
        &self,
        call: agent_core::ToolCall,
        _epoch: u64,
    ) -> Result<serde_json::Value, String> {
        match call.tool_name.as_str() {
            "echo" => Ok(call.arguments),
            other => Err(format!(
                "unsupported tool '{other}' in agent-turn-cli; built-in tools: echo"
            )),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let prompt = resolve_prompt(cli.prompt)?;

    let client = Arc::new(BigModelClient::new(
        Config::new(cli.api_key).with_base_url(cli.base_url),
    ));

    let model_cfg = BigModelAdapterConfig {
        model: cli.model,
        system_prompt: cli.system_prompt,
        max_tokens: cli.max_tokens,
        temperature: cli.temperature,
        top_p: cli.top_p,
    };

    let model = Arc::new(BigModelModelAdapter::new(client).with_config(model_cfg));
    let tools = Arc::new(CliToolExecutor);

    let runtime_cfg = TurnEngineConfig {
        max_parallel_tools: cli.max_parallel_tools,
        retry_policy: RetryPolicy {
            max_retries: cli.max_retries,
            base_delay_ms: cli.base_delay_ms,
        },
    };

    let runtime = TurnRuntime::new(model, tools, runtime_cfg);

    let request = TurnRequest {
        meta: SessionMeta::new(
            cli.session_id.unwrap_or_else(new_id),
            cli.turn_id.unwrap_or_else(new_id),
        ),
        initial_input: InputEnvelope::user_text(prompt),
        transcript: Vec::new(),
    };

    let streams = runtime.run_turn(request).await?;
    stream_events(streams.run, streams.ui, cli.show_run, cli.json).await
}

fn resolve_prompt(cli_prompt: Option<String>) -> Result<String> {
    if let Some(prompt) = cli_prompt {
        let trimmed = prompt.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    if io::stdin().is_terminal() {
        bail!("missing prompt: pass text arg or pipe stdin")
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("failed to read stdin")?;

    let trimmed = buffer.trim().to_string();
    if trimmed.is_empty() {
        bail!("stdin prompt is empty")
    }

    Ok(trimmed)
}

async fn stream_events(
    mut run: RunEventStream,
    mut ui: UiEventStream,
    show_run: bool,
    as_json: bool,
) -> Result<()> {
    let mut run_open = true;
    let mut ui_open = true;
    let mut render_state = UiRenderState::default();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    while run_open || ui_open {
        tokio::select! {
            event = run.next(), if run_open => {
                match event {
                    Some(event) => handle_run_event(event, show_run, as_json)?,
                    None => run_open = false,
                }
            }
            event = ui.next(), if ui_open => {
                match event {
                    Some(event) => handle_ui_event(event, as_json, &mut stdout, &mut stderr, &mut render_state)?,
                    None => ui_open = false,
                }
            }
        }
    }

    finalize_ui_render(as_json, &mut stdout, &mut stderr, &mut render_state)?;

    Ok(())
}

fn handle_run_event(event: RunStreamEvent, show_run: bool, as_json: bool) -> Result<()> {
    if as_json {
        print_json("run", &event)?;
        return Ok(());
    }

    if show_run {
        eprintln!("[run] {event:?}");
    }

    Ok(())
}

#[derive(Default)]
struct UiRenderState {
    printed_text: bool,
    reasoning_started: bool,
    reasoning_open: bool,
}

fn finalize_ui_render(
    as_json: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    state: &mut UiRenderState,
) -> Result<()> {
    if as_json {
        return Ok(());
    }

    let _ = close_reasoning_line(stderr, state)?;

    if state.printed_text {
        writeln!(stdout).context("failed to write trailing newline")?;
        stdout.flush().context("failed to flush stdout")?;
    }

    Ok(())
}

fn close_reasoning_line(stderr: &mut dyn Write, state: &mut UiRenderState) -> Result<bool> {
    if state.reasoning_open {
        writeln!(stderr).context("failed to terminate reasoning line")?;
        stderr.flush().context("failed to flush stderr")?;
        state.reasoning_open = false;
        return Ok(true);
    }
    Ok(false)
}

fn handle_ui_event(
    event: UiThreadEvent,
    as_json: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    state: &mut UiRenderState,
) -> Result<()> {
    if as_json {
        print_json("ui", &event)?;
        return Ok(());
    }

    match event {
        UiThreadEvent::MessageDelta { delta, .. } => {
            if close_reasoning_line(stderr, state)? {
                writeln!(stdout).context("failed to write line break after reasoning")?;
            }
            write!(stdout, "{delta}").context("failed to write message delta")?;
            stdout.flush().context("failed to flush stdout")?;
            state.printed_text = true;
        }
        UiThreadEvent::ReasoningDelta { delta, .. } => {
            if !state.reasoning_started {
                write!(stderr, "[reasoning] ").context("failed to write reasoning prefix")?;
                state.reasoning_started = true;
            }
            write!(stderr, "{delta}").context("failed to write reasoning delta")?;
            stderr.flush().context("failed to flush stderr")?;
            state.reasoning_open = true;
        }
        UiThreadEvent::ToolCallRequested {
            call_id, tool_name, ..
        } => {
            let _ = close_reasoning_line(stderr, state)?;
            writeln!(stderr, "[tool] requested: {tool_name} ({call_id})")
                .context("failed to write tool request")?;
        }
        UiThreadEvent::ToolCallProgress {
            call_id, status, ..
        } => {
            let _ = close_reasoning_line(stderr, state)?;
            writeln!(stderr, "[tool] progress: {call_id} -> {status:?}")
                .context("failed to write tool progress")?;
        }
        UiThreadEvent::ToolCallCompleted { result, .. } => {
            let _ = close_reasoning_line(stderr, state)?;
            writeln!(stderr, "[tool] completed: {}", result.call_id)
                .context("failed to write tool completion")?;
        }
        UiThreadEvent::Warning { message, .. } => {
            let _ = close_reasoning_line(stderr, state)?;
            writeln!(stderr, "[warning] {message}").context("failed to write warning")?;
        }
        UiThreadEvent::Error { message, .. } => {
            let _ = close_reasoning_line(stderr, state)?;
            writeln!(stderr, "[error] {message}").context("failed to write error")?;
        }
        UiThreadEvent::Done { summary, stats, .. } => {
            let _ = close_reasoning_line(stderr, state)?;
            if let Some(summary) = summary {
                writeln!(stderr, "[done] summary: {summary}")
                    .context("failed to write done summary")?;
            }
            writeln!(
                stderr,
                "[done] stats: tools={} input_tokens={} output_tokens={}",
                stats.tool_calls_count, stats.total_input_tokens, stats.total_output_tokens
            )
            .context("failed to write done stats")?;
        }
    }

    Ok(())
}

fn print_json<T: Serialize>(stream: &str, event: &T) -> Result<()> {
    let line = serde_json::to_string(&serde_json::json!({
        "stream": stream,
        "event": event,
    }))
    .context("failed to serialize event")?;
    println!("{line}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::TurnStats;

    fn ui_event_turn_id() -> String {
        "turn-1".to_string()
    }

    #[test]
    fn reasoning_streams_on_single_prefixed_line() {
        let mut stdout = Vec::<u8>::new();
        let mut stderr = Vec::<u8>::new();
        let mut state = UiRenderState::default();

        handle_ui_event(
            UiThreadEvent::ReasoningDelta {
                turn_id: ui_event_turn_id(),
                delta: "step1 ".to_string(),
            },
            false,
            &mut stdout,
            &mut stderr,
            &mut state,
        )
        .expect("first delta");
        handle_ui_event(
            UiThreadEvent::ReasoningDelta {
                turn_id: ui_event_turn_id(),
                delta: "step2".to_string(),
            },
            false,
            &mut stdout,
            &mut stderr,
            &mut state,
        )
        .expect("second delta");
        handle_ui_event(
            UiThreadEvent::Done {
                turn_id: ui_event_turn_id(),
                summary: None,
                stats: TurnStats::default(),
            },
            false,
            &mut stdout,
            &mut stderr,
            &mut state,
        )
        .expect("done");
        finalize_ui_render(false, &mut stdout, &mut stderr, &mut state).expect("finalize");

        let stderr = String::from_utf8(stderr).expect("stderr utf8");
        assert!(stderr.starts_with("[reasoning] step1 step2\n"));
        assert_eq!(
            stderr,
            "[reasoning] step1 step2\n[done] stats: tools=0 input_tokens=0 output_tokens=0\n"
        );
    }

    #[test]
    fn message_delta_closes_reasoning_line_before_stdout_stream() {
        let mut stdout = Vec::<u8>::new();
        let mut stderr = Vec::<u8>::new();
        let mut state = UiRenderState::default();

        handle_ui_event(
            UiThreadEvent::ReasoningDelta {
                turn_id: ui_event_turn_id(),
                delta: "thinking".to_string(),
            },
            false,
            &mut stdout,
            &mut stderr,
            &mut state,
        )
        .expect("reasoning");
        handle_ui_event(
            UiThreadEvent::MessageDelta {
                turn_id: ui_event_turn_id(),
                delta: "answer".to_string(),
            },
            false,
            &mut stdout,
            &mut stderr,
            &mut state,
        )
        .expect("message");
        finalize_ui_render(false, &mut stdout, &mut stderr, &mut state).expect("finalize");

        let stdout = String::from_utf8(stdout).expect("stdout utf8");
        let stderr = String::from_utf8(stderr).expect("stderr utf8");

        assert_eq!(stderr, "[reasoning] thinking\n");
        assert_eq!(stdout, "\nanswer\n");
    }
}
