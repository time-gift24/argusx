use std::path::Path;
use anyhow::{Result, Context};
use std::fs;

use super::models::AgentDefinition;

pub fn load_agents(dir: &Path) -> Result<Vec<AgentDefinition>> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut agents = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "toml") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {:?}", path))?;

            let agent: AgentDefinition = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {:?}", path))?;

            agents.push(agent);
        }
    }

    Ok(agents)
}
