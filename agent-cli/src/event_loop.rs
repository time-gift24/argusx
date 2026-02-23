use crossterm::event::{KeyCode, KeyEvent, Event, poll, read};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use std::time::Duration;

use crate::app::{AppState, Role};
use crate::runtime::AppEvent;
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
        AppEvent::ReasoningDelta { delta: _ } => {
            // TODO: Store reasoning separately for fold toggle
        }
        AppEvent::TurnFinished { turn_id: _, failed } => {
            app.active_turn = None;
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
        _ => {}
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
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Channel for receiving events from agent stream
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

    // Spawn task to pump agent stream (placeholder - real implementation would wire this up)
    let _agent = agent.clone();
    let _session_id = app.session_id.clone();
    let _tx = tx.clone();
    // TODO: Wire up actual stream pumping with pump_stream()

    loop {
        // Render
        terminal.draw(|frame| draw(frame, app))?;

        // Check for events from agent (non-blocking)
        if let Ok(event) = rx.try_recv() {
            process_app_event(app, event);
        }

        // Poll for keyboard input (non-blocking)
        if poll(Duration::from_millis(100))? {
            if let Ok(Event::Key(key)) = read() {
                match handle_key_event(app, key) {
                    LoopAction::Quit => break,
                    LoopAction::Submit => {
                        if let Some(_cmd) = app.submit_input() {
                            // TODO: Send message to agent and pump stream
                            // let agent = agent.clone();
                            // let session_id = app.session_id.clone();
                            // let tx = tx.clone();
                            // tokio::spawn(async move { ... });
                        }
                    }
                    LoopAction::None => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn esc_requests_exit() {
        let mut app = AppState::new("s-1".into());
        let action = handle_key_event(
            &mut app,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
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
}
