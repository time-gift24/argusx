use std::net::SocketAddr;

use anyhow::{Context, Result};
use llm_gateway::{GatewayState, app};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listen_addr = std::env::var("GATEWAY_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse::<SocketAddr>()
        .context("invalid GATEWAY_LISTEN_ADDR")?;

    let client = llm_client::LlmClient::builder()
        .with_default_bigmodel_from_env()
        .context("failed to create LLM client")?
        .build()
        .context("failed to build LLM client")?;

    let state = GatewayState::new(client);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    tracing::info!(addr = %listen_addr, "llm-gateway listening");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
