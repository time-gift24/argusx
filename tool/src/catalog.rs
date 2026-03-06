use std::sync::Arc;

use argus_core::Builtin;

use crate::Tool;

pub struct BuiltinRegistration {
    pub builtin: Builtin,
    pub tool: Arc<dyn Tool>,
}

impl BuiltinRegistration {
    pub fn new(builtin: Builtin, tool: Arc<dyn Tool>) -> Self {
        Self { builtin, tool }
    }
}
