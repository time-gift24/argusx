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

    pub fn transition_to(&mut self, next: ThreadStatus) -> Result<(), &'static str> {
        let legal = matches!(
            (self.status, next),
            // Normal progression
            (ThreadStatus::Pending, ThreadStatus::Running)
                | (ThreadStatus::Pending, ThreadStatus::Failed)
                | (ThreadStatus::Running, ThreadStatus::Succeeded)
                | (ThreadStatus::Running, ThreadStatus::Failed)
                | (ThreadStatus::Running, ThreadStatus::Cancelled)
                | (ThreadStatus::Running, ThreadStatus::Closing)
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
            return Err("illegal transition");
        }

        self.status = next;
        Ok(())
    }
}
