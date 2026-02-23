use clap::Parser;
use agent_cli::cli::CliArgs;
use agent_cli::app::AppState;
use agent_cli::event_loop::run_tui_loop;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    let client = std::sync::Arc::new(bigmodel_api::BigModelClient::new(
        bigmodel_api::Config::new(args.api_key.clone()).with_base_url(args.base_url.clone()),
    ));

    let model_cfg = agent_turn::adapters::bigmodel::BigModelAdapterConfig {
        model: args.model.clone(),
        system_prompt: args.system_prompt.clone(),
        max_tokens: args.max_tokens,
        temperature: args.temperature,
        top_p: args.top_p,
    };

    let model = std::sync::Arc::new(
        agent_turn::adapters::bigmodel::BigModelModelAdapter::new(client).with_config(model_cfg),
    );

    let mut builder = agent::AgentBuilder::new().model(model);
    if let Some(store_dir) = args.store_dir.clone() {
        builder = builder.store_dir(store_dir);
    }
    let agent = builder.build().await?;
    let agent = std::sync::Arc::new(agent);

    let gateway = AgentSessionGateway::new(&agent);
    let session_id = agent_cli::session::resolve_session_id(&gateway, args.session.as_deref()).await?;

    let mut app = AppState::new(session_id);
    run_tui_loop(agent, &mut app, args.debug_events).await?;

    Ok(())
}

struct AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    agent: &'a agent::Agent<L>,
}

impl<'a, L> AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub fn new(agent: &'a agent::Agent<L>) -> Self {
        Self { agent }
    }
}

#[async_trait::async_trait]
impl<'a, L> agent_cli::session::SessionGateway for AgentSessionGateway<'a, L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    async fn create_session(&self) -> anyhow::Result<String> {
        Ok(self.agent.create_session(None, Some("agent-cli".into())).await?)
    }

    async fn session_exists(&self, session_id: &str) -> anyhow::Result<bool> {
        Ok(self.agent.get_session(session_id).await?.is_some())
    }
}
