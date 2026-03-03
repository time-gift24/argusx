pub mod config;
pub mod error;
pub mod gateway;
pub mod proxy;
pub mod store;
pub mod gateway;

pub use store::{CookieData, CookieStore};

use std::sync::Arc;
use std::net::SocketAddr;

pub struct CookieGateway {
    store: Arc<CookieStore>,
}

impl CookieGateway {
    pub fn new(store: CookieStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub fn store(&self) -> &CookieStore {
        &self.store
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = gateway::GatewayState {
            store: self.store.clone(),
        };
        let app = gateway::app(state);
        let addr = SocketAddr::from(([127, 0, 0, 1], 3456));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Cookie gateway listening on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
