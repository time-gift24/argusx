use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalStepPolicy {
    /// Disable tools on the final step and let the model produce a text response.
    ForceText,
    /// Fail the turn immediately when max_steps is reached.
    Fail,
}

#[derive(Debug, Clone, Copy)]
pub struct TurnOptions {
    /// Per-tool execution timeout. Default is 30 s (was 5 s in the original single-field struct).
    pub tool_timeout: Duration,
    /// How long to wait for the model stream to start after calling `start()`.
    pub model_start_timeout: Duration,
    /// How long to wait between consecutive stream events before treating the
    /// stream as stalled.
    pub stream_idle_timeout: Duration,
    /// Total wall-clock budget for the entire turn.
    pub turn_deadline: Duration,
    /// Maximum number of tool-call steps allowed. On reaching this limit, the
    /// policy in `final_step_policy` is applied.
    pub max_steps: u32,
    /// Policy applied when `max_steps` is reached.
    pub final_step_policy: FinalStepPolicy,
}

impl Default for TurnOptions {
    fn default() -> Self {
        Self {
            tool_timeout: Duration::from_secs(30),
            model_start_timeout: Duration::from_secs(10),
            stream_idle_timeout: Duration::from_secs(30),
            turn_deadline: Duration::from_secs(300),
            max_steps: 8,
            final_step_policy: FinalStepPolicy::ForceText,
        }
    }
}
