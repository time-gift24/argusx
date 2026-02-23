use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::time::Duration;

use crate::app::{AppState, Role};
use crate::runtime::{pump_stream, AppEvent};
use crate::ui::draw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    None,
    Submit,
    Quit,
}

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> LoopAction {
    match key.code {
        KeyCode::Esc => LoopAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => LoopAction::Quit,
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

/// Process an AppEvent from the agent runtime
pub fn process_app_event(app: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::AssistantDelta { delta } => {
            // Append to last assistant message or create new one
            if let Some(last) = app.messages.last_mut() {
                if last.role == Role::Assistant {
                    last.text.push_str(&delta);
                } else {
                    app.messages.push(crate::app::MessageItem {
                        role: Role::Assistant,
                        text: delta,
                    });
                }
            } else {
                app.messages.push(crate::app::MessageItem {
                    role: Role::Assistant,
                    text: delta,
                });
            }
        }
        AppEvent::ReasoningDelta { delta } => {
            // Store reasoning separately for fold toggle
            app.reasoning_text.push_str(&delta);
        }
        AppEvent::ToolRequested { call_id, tool_name } => {
            app.tool_progress.push(crate::app::ToolProgressItem {
                tool_name,
                status: format!("requested: {}", call_id),
            });
        }
        AppEvent::ToolProgress { call_id: _, status } => {
            // Update the latest tool progress entry
            if let Some(last) = app.tool_progress.last_mut() {
                last.status = status;
            }
        }
        AppEvent::ToolCompleted { call_id: _ } => {
            // Mark the latest tool as completed
            if let Some(last) = app.tool_progress.last_mut() {
                if !last.status.starts_with("completed") {
                    last.status = format!("completed: {}", last.status);
                }
            }
        }
        AppEvent::TurnFinished { turn_id: _, failed } => {
            app.active_turn = None;
            // Clear reasoning and tool progress for next turn
            app.reasoning_text.clear();
            app.tool_progress.clear();
            if failed {
                app.last_warning = Some("turn failed".to_string());
            }
        }
        AppEvent::Warning { message } => {
            app.last_warning = Some(message);
        }
        AppEvent::Error { message } => {
            app.last_warning = Some(format!("error: {}", message));
        }
    }
}

/// RAII guard to ensure raw mode is disabled on drop
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Run the TUI event loop
pub async fn run_tui_loop<L>(
    agent: std::sync::Arc<agent::Agent<L>>,
    app: &mut AppState,
    _debug_events: bool,
) -> anyhow::Result<()>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io::stdout;

    enable_raw_mode()?;
    // RAII guard ensures raw mode is disabled even on error
    let _raw_guard = RawModeGuard;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Channel for receiving events from agent stream
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

    // Keep track if we have an active stream task
    let mut active_stream_task: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        // Render
        terminal.draw(|frame| draw(frame, app))?;

        // Check for events from agent (non-blocking)
        if let Ok(event) = rx.try_recv() {
            process_app_event(app, event);
        }

        // Check if previous stream task completed
        if let Some(task) = active_stream_task.take() {
            if task.is_finished() {
                // Task done, we can accept new submissions
            } else {
                active_stream_task = Some(task);
            }
        }

        // Poll for keyboard input (non-blocking)
        if poll(Duration::from_millis(100))? {
            if let Ok(Event::Key(key)) = read() {
                match handle_key_event(app, key) {
                    LoopAction::Quit => break,
                    LoopAction::Submit => {
                        if let Some(cmd) = app.submit_input() {
                            // Start a new chat stream
                            let agent = agent.clone();
                            let session_id = cmd.session_id;
                            let message = cmd.message;
                            let tx = tx.clone();

                            // Spawn task to pump agent stream
                            let handle = tokio::spawn(async move {
                                match agent.chat_stream(&session_id, &message).await {
                                    Ok(stream) => {
                                        pump_stream(stream, tx).await;
                                    }
                                    Err(e) => {
                                        let _ = tx.send(AppEvent::Error {
                                            message: format!("chat error: {}", e),
                                        });
                                    }
                                }
                            });
                            active_stream_task = Some(handle);
                        }
                    }
                    LoopAction::None => {}
                }
            }
        }
    }

    // Drop guard to ensure raw mode is disabled
    drop(_raw_guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn esc_requests_exit() {
        let mut app = AppState::new("s-1".into());
        let action = handle_key_event(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(action, LoopAction::Quit));
    }

    #[test]
    fn ctrl_c_requests_exit() {
        let mut app = AppState::new("s-1".into());
        let action = handle_key_event(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, LoopAction::Quit));
    }

    #[test]
    fn tab_toggles_reasoning_visibility() {
        let mut app = AppState::new("s-1".into());
        assert!(app.show_reasoning);
        let _ = handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(!app.show_reasoning);
    }

    #[test]
    fn reasoning_delta_updates_reasoning_text() {
        let mut app = AppState::new("s-1".into());
        process_app_event(
            &mut app,
            AppEvent::ReasoningDelta {
                delta: "thinking...".to_string(),
            },
        );
        assert_eq!(app.reasoning_text, "thinking...");
    }

    #[test]
    fn tool_requested_adds_tool_progress() {
        let mut app = AppState::new("s-1".into());
        process_app_event(
            &mut app,
            AppEvent::ToolRequested {
                call_id: "call-123".to_string(),
                tool_name: "bash".to_string(),
            },
        );
        assert_eq!(app.tool_progress.len(), 1);
        assert_eq!(app.tool_progress[0].tool_name, "bash");
    }

    #[test]
    fn tool_progress_updates_status() {
        let mut app = AppState::new("s-1".into());
        // First add a tool request
        process_app_event(
            &mut app,
            AppEvent::ToolRequested {
                call_id: "call-123".to_string(),
                tool_name: "bash".to_string(),
            },
        );
        // Then update progress
        process_app_event(
            &mut app,
            AppEvent::ToolProgress {
                call_id: "call-123".to_string(),
                status: "Running command...".to_string(),
            },
        );
        assert_eq!(app.tool_progress[0].status, "Running command...");
    }

    #[test]
    fn turn_finished_clears_reasoning_and_tool_progress() {
        let mut app = AppState::new("s-1".into());
        app.reasoning_text = "some reasoning".to_string();
        app.tool_progress.push(crate::app::ToolProgressItem {
            tool_name: "bash".to_string(),
            status: "done".to_string(),
        });
        app.active_turn = Some(crate::app::ActiveTurn::default());

        process_app_event(
            &mut app,
            AppEvent::TurnFinished {
                turn_id: "t1".to_string(),
                failed: false,
            },
        );

        assert!(app.active_turn.is_none());
        assert!(app.reasoning_text.is_empty());
        assert!(app.tool_progress.is_empty());
    }

    #[test]
    fn turn_finished_with_failure_sets_warning() {
        let mut app = AppState::new("s-1".into());
        app.active_turn = Some(crate::app::ActiveTurn::default());

        process_app_event(
            &mut app,
            AppEvent::TurnFinished {
                turn_id: "t1".to_string(),
                failed: true,
            },
        );

        assert!(app.active_turn.is_none());
        assert_eq!(app.last_warning, Some("turn failed".to_string()));
    }
}
