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
