#[test]
fn exports_builder_api() {
    let _ = agent_center::AgentCenter::builder();
}

#[test]
fn builder_creates_agent_center_with_custom_limits() {
    use agent_center::AgentCenter;

    let center = AgentCenter::builder()
        .max_concurrent(5)
        .max_depth(2)
        .build()
        .expect("builder should succeed");

    // Verify center was created (guards are not publicly accessible, but build succeeded)
    let _ = center;
}

#[test]
fn builder_uses_defaults() {
    use agent_center::AgentCenter;

    let center = AgentCenter::builder().build().expect("builder should succeed");
    let _ = center;
}
