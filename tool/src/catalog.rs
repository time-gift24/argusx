use std::sync::Arc;

use argus_core::Builtin;

use crate::Tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveToolPolicy {
    pub allow_parallel: bool,
    pub max_concurrency: usize,
}

pub struct BuiltinRegistration {
    pub builtin: Builtin,
    pub tool: Arc<dyn Tool>,
    pub policy: EffectiveToolPolicy,
}

impl BuiltinRegistration {
    pub fn new(builtin: Builtin, tool: Arc<dyn Tool>, policy: EffectiveToolPolicy) -> Self {
        Self {
            builtin,
            tool,
            policy,
        }
    }
}
