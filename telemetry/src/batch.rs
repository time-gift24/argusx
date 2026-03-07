use crate::{EventPriority, TelemetryConfig, TelemetryRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchEnqueueResult {
    Queued,
    FlushRequired,
    DroppedLowPriority,
}

pub struct BatchQueue {
    config: TelemetryConfig,
    high: Vec<TelemetryRecord>,
    low: Vec<TelemetryRecord>,
}

impl BatchQueue {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            high: Vec::new(),
            low: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, record: TelemetryRecord) -> BatchEnqueueResult {
        let total = self.high.len() + self.low.len();

        match record.event_priority {
            EventPriority::High => {
                // If queue is full, drop low priority to make room for high priority
                if total >= self.config.max_in_memory_events && !self.low.is_empty() {
                    self.low.pop();
                }

                self.high.push(record);

                if self.high.len() >= self.config.high_priority_batch_size {
                    BatchEnqueueResult::FlushRequired
                } else {
                    BatchEnqueueResult::Queued
                }
            }
            EventPriority::Low => {
                // Drop low priority if queue is full
                if total >= self.config.max_in_memory_events {
                    BatchEnqueueResult::DroppedLowPriority
                } else {
                    self.low.push(record);
                    BatchEnqueueResult::Queued
                }
            }
        }
    }

    pub fn drain_high(&mut self) -> Vec<TelemetryRecord> {
        std::mem::take(&mut self.high)
    }

    pub fn drain_low(&mut self) -> Vec<TelemetryRecord> {
        std::mem::take(&mut self.low)
    }

    pub fn high_len(&self) -> usize {
        self.high.len()
    }

    pub fn low_len(&self) -> usize {
        self.low.len()
    }

    pub fn should_flush_high(&self) -> bool {
        self.high.len() >= self.config.high_priority_batch_size
    }

    pub fn should_flush_low(&self) -> bool {
        self.low.len() >= self.config.low_priority_batch_size
    }
}
