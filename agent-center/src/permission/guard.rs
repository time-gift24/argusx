use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("Maximum concurrent agents exceeded")]
    MaxConcurrentExceeded,

    #[error("Maximum depth exceeded: parent_depth={parent_depth}, max_depth={max_depth}")]
    MaxDepthExceeded { parent_depth: u32, max_depth: u32 },
}

pub struct SpawnGuards {
    max_concurrent: usize,
    max_depth: u32,
    running: Arc<AtomicUsize>,
}

#[derive(Debug)]
pub struct SpawnReservation {
    running: Arc<AtomicUsize>,
}

impl SpawnGuards {
    pub fn new(max_concurrent: usize, max_depth: u32) -> Self {
        Self {
            max_concurrent,
            max_depth,
            running: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn reserve(&self, parent_depth: u32) -> Result<SpawnReservation, GuardError> {
        if parent_depth >= self.max_depth {
            return Err(GuardError::MaxDepthExceeded {
                parent_depth,
                max_depth: self.max_depth,
            });
        }

        let current = self.running.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_concurrent {
            self.running.fetch_sub(1, Ordering::SeqCst);
            return Err(GuardError::MaxConcurrentExceeded);
        }

        Ok(SpawnReservation {
            running: Arc::clone(&self.running),
        })
    }
}

impl Drop for SpawnReservation {
    fn drop(&mut self) {
        self.running.fetch_sub(1, Ordering::SeqCst);
    }
}
