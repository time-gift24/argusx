use super::models::AgentDefinition;
use anyhow::{bail, Result};

pub fn validate(def: &AgentDefinition) -> Result<()> {
    if def.name.trim().is_empty() {
        bail!("Agent name cannot be empty");
    }

    if def.version.trim().is_empty() {
        bail!("Agent version cannot be empty");
    }

    if def.prompt.trim().is_empty() {
        bail!("Agent prompt cannot be empty");
    }

    Ok(())
}
