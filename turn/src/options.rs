use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct TurnOptions {
    pub tool_timeout: Duration,
}

impl Default for TurnOptions {
    fn default() -> Self {
        Self {
            tool_timeout: Duration::from_secs(5),
        }
    }
}
