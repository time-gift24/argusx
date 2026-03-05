use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::models::AgentDefinition;

pub fn load_agents(dir: &Path) -> Result<Vec<AgentDefinition>> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut agents = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "toml") {
            let content =
                fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;

            let agent: AgentDefinition =
                toml::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))?;

            super::validator::validate(&agent)
                .with_context(|| format!("Invalid agent config {:?}", path))?;

            agents.push(agent);
        }
    }

    Ok(agents)
}
