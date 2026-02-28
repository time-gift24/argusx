use std::net::SocketAddr;

use anyhow::{Context, Result};
use llm_client::providers::{BigModelConfig, BigModelHttpClient};
use llm_gateway::{GatewayState, app};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listen_addr = std::env::var("GATEWAY_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse::<SocketAddr>()
        .context("invalid GATEWAY_LISTEN_ADDR")?;

    let api_key = std::env::var("BIGMODEL_API_KEY").context("BIGMODEL_API_KEY is required")?;

    let base_url = std::env::var("BIGMODEL_BASE_URL")
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".to_string());

    let client = BigModelHttpClient::new(BigModelConfig { base_url, api_key });
    let state = GatewayState::new(client);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    tracing::info!(addr = %listen_addr, "llm-gateway listening");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
