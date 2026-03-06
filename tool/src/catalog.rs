use std::sync::Arc;

use argus_core::Builtin;

use crate::{Tool, mcp::McpClient};

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

pub struct McpRegistration {
    pub server_label: String,
    pub client: Arc<McpClient>,
    pub policy: EffectiveToolPolicy,
}

impl McpRegistration {
    pub fn new(server_label: String, client: Arc<McpClient>, policy: EffectiveToolPolicy) -> Self {
        Self {
            server_label,
            client,
            policy,
        }
    }
}
