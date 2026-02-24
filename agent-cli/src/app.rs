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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProgressItem {
    pub call_id: String,
    pub tool_name: String,
    pub status: String,
}

pub struct AppState {
    pub session_id: String,
    pub input: String,
    pub messages: Vec<MessageItem>,
    pub active_turn: Option<ActiveTurn>,
    pub last_warning: Option<String>,
    pub show_reasoning: bool,
    pub reasoning_text: String,
    pub tool_progress: Vec<ToolProgressItem>,
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
            reasoning_text: String::new(),
            tool_progress: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
