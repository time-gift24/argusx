use std::net::SocketAddr;
use std::collections::HashMap;

use anyhow::{Context, Result};
use llm_gateway::{GatewayState, app};
use llm_provider::bigmodel::{BigModelAdapter, BigModelConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listen_addr = std::env::var("GATEWAY_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse::<SocketAddr>()
        .context("invalid GATEWAY_LISTEN_ADDR")?;

    let api_key = std::env::var("BIGMODEL_API_KEY").context("BIGMODEL_API_KEY is required")?;
    let base_url = std::env::var("BIGMODEL_BASE_URL").context("BIGMODEL_BASE_URL is required")?;
    let provider_cfg = BigModelConfig::new(base_url, api_key, HashMap::new())
        .context("failed to create BigModel config")?;

    let client = llm_client::LlmClient::builder()
        .register_adapter(Arc::new(BigModelAdapter::new(provider_cfg)))
        .default_adapter("bigmodel")
        .build()
        .context("failed to build LLM client")?;

    let state = GatewayState::new(client);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    tracing::info!(addr = %listen_addr, "llm-gateway listening");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
