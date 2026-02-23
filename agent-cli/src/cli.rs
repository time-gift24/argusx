use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "agent-cli", about = "Terminal chat UI for agent facade")]
pub struct CliArgs {
    #[arg(long, env = "BIGMODEL_API_KEY")]
    pub api_key: String,
    #[arg(long, env = "BIGMODEL_BASE_URL", default_value = "https://open.bigmodel.cn/api/paas/v4")]
    pub base_url: String,
    #[arg(long, default_value = "glm-4.5")]
    pub model: String,
    #[arg(long)]
    pub system_prompt: Option<String>,
    #[arg(long)]
    pub max_tokens: Option<i32>,
    #[arg(long)]
    pub temperature: Option<f32>,
    #[arg(long)]
    pub top_p: Option<f32>,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub store_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub debug_events: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_new_session_mode() {
        let args = ["agent-cli", "--api-key", "k"];
        let cfg = CliArgs::parse_from(args);
        assert!(cfg.session.is_none());
    }

    #[test]
    fn parse_accepts_session_resume() {
        let args = ["agent-cli", "--api-key", "k", "--session", "s-1"];
        let cfg = CliArgs::parse_from(args);
        assert_eq!(cfg.session.as_deref(), Some("s-1"));
    }
}
