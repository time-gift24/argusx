use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use anyhow::{anyhow, Result};

pub struct SpawnGuards {
    max_concurrent: usize,
    max_depth: u32,
    running: Arc<AtomicUsize>,
}

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

    pub fn reserve(&self, parent_depth: u32) -> Result<SpawnReservation> {
        if parent_depth >= self.max_depth {
            return Err(anyhow!("max depth exceeded"));
        }

        let current = self.running.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_concurrent {
            self.running.fetch_sub(1, Ordering::SeqCst);
            return Err(anyhow!("max concurrent exceeded"));
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
