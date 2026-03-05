use std::collections::HashSet;

use crate::command::DomainCommand;

#[derive(Debug, Default)]
pub struct CommandNormalizer {
    seen_ids: HashSet<String>,
}

impl CommandNormalizer {
    pub fn normalize(&mut self, cmd: DomainCommand) -> Option<DomainCommand> {
        let id = cmd.id().to_string();
        if !self.seen_ids.insert(id) {
            return None;
        }
        Some(cmd)
    }
}

#[cfg(test)]
mod tests {
    use agent_core::RuntimeEvent;

    use crate::command::DomainCommand;

    use super::CommandNormalizer;

    #[test]
    fn normalizer_drops_duplicate_event_ids() {
        let mut n = CommandNormalizer::default();
        let first = DomainCommand::from_runtime(RuntimeEvent::FatalError {
            event_id: "e1".into(),
            message: "x".into(),
        });
        let second = first.clone();

        assert!(n.normalize(first).is_some());
        assert!(n.normalize(second).is_none());
    }
}

