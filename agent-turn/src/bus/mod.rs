use std::collections::VecDeque;

use crate::command::DomainCommand;

#[derive(Debug, Clone)]
pub struct BusConfig {
    pub command_capacity: usize,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            command_capacity: 1024,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BusError {
    #[error("command queue is full")]
    CommandQueueFull,
}

#[derive(Debug)]
pub struct EventBus {
    config: BusConfig,
    command_queue: VecDeque<DomainCommand>,
}

impl EventBus {
    pub fn new(config: BusConfig) -> Self {
        Self {
            config,
            command_queue: VecDeque::new(),
        }
    }

    pub fn enqueue_command(&mut self, cmd: DomainCommand) -> Result<(), BusError> {
        if self.command_queue.len() >= self.config.command_capacity {
            return Err(BusError::CommandQueueFull);
        }
        self.command_queue.push_back(cmd);
        Ok(())
    }

    pub fn dequeue_command(&mut self) -> Option<DomainCommand> {
        self.command_queue.pop_front()
    }

    pub fn drain_commands_for_test(&mut self) -> Vec<DomainCommand> {
        self.command_queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::command::DomainCommand;

    use super::{BusConfig, EventBus};

    #[test]
    fn bus_pump_preserves_fifo_order() {
        let mut bus = EventBus::new(BusConfig::default());
        bus.enqueue_command(DomainCommand::Noop { id: "c1".into() })
            .expect("enqueue c1");
        bus.enqueue_command(DomainCommand::Noop { id: "c2".into() })
            .expect("enqueue c2");

        let drained = bus.drain_commands_for_test();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].id(), "c1");
        assert_eq!(drained[1].id(), "c2");
    }
}
