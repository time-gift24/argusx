#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Closing,
    Closed,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("illegal transition: {from:?} -> {to:?}")]
    IllegalTransition { from: ThreadStatus, to: ThreadStatus },
}

pub struct ThreadStateMachine {
    status: ThreadStatus,
}

impl ThreadStateMachine {
    pub fn new(status: ThreadStatus) -> Self {
        Self { status }
    }

    pub fn status(&self) -> ThreadStatus {
        self.status
    }

    pub fn transition_to(&mut self, next: ThreadStatus) -> Result<(), LifecycleError> {
        let legal = matches!(
            (self.status, next),
            // Normal progression
            (ThreadStatus::Pending, ThreadStatus::Running)
                | (ThreadStatus::Pending, ThreadStatus::Failed)
                | (ThreadStatus::Running, ThreadStatus::Succeeded)
                | (ThreadStatus::Running, ThreadStatus::Failed)
                | (ThreadStatus::Running, ThreadStatus::Cancelled)
                | (ThreadStatus::Running, ThreadStatus::Closing)
                // Force close: Running -> Closed (bypasses Closing state)
                | (ThreadStatus::Running, ThreadStatus::Closed)
                | (ThreadStatus::Closing, ThreadStatus::Closed)
                | (ThreadStatus::Closing, ThreadStatus::Failed)
                // Idempotent transitions (terminal states stay terminal)
                | (ThreadStatus::Succeeded, ThreadStatus::Succeeded)
                | (ThreadStatus::Failed, ThreadStatus::Failed)
                | (ThreadStatus::Cancelled, ThreadStatus::Cancelled)
                | (ThreadStatus::Closed, ThreadStatus::Closed)
                // Idempotent running (for retry scenarios)
                | (ThreadStatus::Running, ThreadStatus::Running)
        );

        if !legal {
            return Err(LifecycleError::IllegalTransition {
                from: self.status,
                to: next,
            });
        }

        self.status = next;
        Ok(())
    }
}
