pub mod command_bus;
pub mod config;
pub mod error;
pub mod gateway;
pub mod proxy;
pub mod store;
pub mod tool;

pub use store::{CachedCookies, CookieData, CookieStore};
pub use tool::{CookieFetchOutput, CookieFetchSource};

use std::net::SocketAddr;
use std::sync::Arc;

pub struct CookieGateway {
    state: gateway::GatewayState,
}

impl CookieGateway {
    pub fn new(store: CookieStore) -> Self {
        let state = gateway::GatewayState::with_store(Arc::new(store));
        Self { state }
    }

    pub fn store(&self) -> &CookieStore {
        self.state.store.as_ref()
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = gateway::app(self.state.clone());
        let addr = SocketAddr::from(([127, 0, 0, 1], 3456));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Cookie gateway listening on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
