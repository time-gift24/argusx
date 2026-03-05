use agent_center::config::loader::load_agents;
use agent_center::config::validator::validate;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn rejects_invalid_agent_config() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let config_dir = temp.path().join(".agents");
    std::fs::create_dir(&config_dir)?;

    // Write malformed TOML (missing required field)
    let bad_config = config_dir.join("bad.toml");
    let mut file = std::fs::File::create(&bad_config)?;
    file.write_all(
        br#"
name = "test-agent"
# missing version field
"#,
    )?;

    let result = load_agents(&config_dir);
    assert!(result.is_err(), "should reject config with missing fields");

    Ok(())
}

#[test]
fn loads_valid_agent_config() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let config_dir = temp.path().join(".agents");
    std::fs::create_dir(&config_dir)?;

    let good_config = config_dir.join("test.toml");
    let mut file = std::fs::File::create(&good_config)?;
    file.write_all(
        br#"
name = "test-agent"
version = "1.0.0"
prompt = "You are a test agent"
"#,
    )?;

    let agents = load_agents(&config_dir)?;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "test-agent");
    assert_eq!(agents[0].version, "1.0.0");

    // Validate the loaded config
    validate(&agents[0])?;

    Ok(())
}

#[test]
fn validates_required_fields() -> anyhow::Result<()> {
    use agent_center::config::models::AgentDefinition;

    let missing_name = AgentDefinition {
        name: "".to_string(),
        version: "1.0.0".to_string(),
        prompt: "test".to_string(),
        tools: vec![],
        max_concurrent: None,
        max_depth: None,
    };

    let result = validate(&missing_name);
    assert!(result.is_err(), "should reject config with empty name");

    Ok(())
}
