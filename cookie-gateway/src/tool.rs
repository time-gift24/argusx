use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::error::CookieGatewayError;
use crate::store::CachedCookies;
use crate::{CookieData, CookieStore};

#[async_trait]
pub trait CookieCommandClient: Send + Sync {
    async fn request_cookies(
        &self,
        domain: &str,
        timeout: Duration,
    ) -> Result<Vec<CookieData>, CookieGatewayError>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CookieFetchSource {
    Cache,
    Refresh,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookieFetchOutput {
    pub domain: String,
    pub source: CookieFetchSource,
    pub age_ms: u64,
    pub fetched_at_unix_ms: u64,
    pub count: usize,
    pub cookies: Vec<CookieData>,
}

#[derive(Clone)]
pub struct CookieFetchTool {
    store: Arc<CookieStore>,
    command_client: Arc<dyn CookieCommandClient>,
    refresh_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    command_timeout: Duration,
}

impl CookieFetchTool {
    pub fn new(
        store: Arc<CookieStore>,
        command_client: Arc<dyn CookieCommandClient>,
        command_timeout: Duration,
    ) -> Self {
        Self {
            store,
            command_client,
            refresh_locks: Arc::new(Mutex::new(HashMap::new())),
            command_timeout,
        }
    }

    pub async fn fetch(
        &self,
        domain: &str,
        refresh_after: Duration,
    ) -> Result<CookieFetchOutput, CookieGatewayError> {
        if let Some(cached) = self.store.get_cached(domain).await {
            if is_fresh(&cached, refresh_after) {
                return Ok(build_output(domain, cached, CookieFetchSource::Cache));
            }
        }

        let domain_lock = self.domain_lock(domain).await;
        let _guard = domain_lock.lock().await;

        if let Some(cached) = self.store.get_cached(domain).await {
            if is_fresh(&cached, refresh_after) {
                return Ok(build_output(domain, cached, CookieFetchSource::Cache));
            }
        }

        let cookies = self
            .command_client
            .request_cookies(domain, self.command_timeout)
            .await?;
        self.store.store_cookies(domain, cookies).await;

        let refreshed = self.store.get_cached(domain).await.ok_or_else(|| {
            CookieGatewayError::NoCookiesFound {
                domain: domain.to_string(),
            }
        })?;

        Ok(build_output(domain, refreshed, CookieFetchSource::Refresh))
    }

    async fn domain_lock(&self, domain: &str) -> Arc<Mutex<()>> {
        let mut locks = self.refresh_locks.lock().await;
        locks
            .entry(domain.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

fn is_fresh(cached: &CachedCookies, refresh_after: Duration) -> bool {
    if refresh_after.is_zero() {
        return false;
    }

    age_ms(cached.fetched_at) <= refresh_after.as_millis() as u64
}

fn build_output(
    domain: &str,
    cached: CachedCookies,
    source: CookieFetchSource,
) -> CookieFetchOutput {
    CookieFetchOutput {
        domain: domain.to_string(),
        source,
        age_ms: age_ms(cached.fetched_at),
        fetched_at_unix_ms: unix_ms(cached.fetched_at),
        count: cached.cookies.len(),
        cookies: cached.cookies,
    }
}

fn age_ms(time: SystemTime) -> u64 {
    let now = SystemTime::now();
    now.duration_since(time).unwrap_or_default().as_millis() as u64
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
