use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Whitelist of allowed domains for cookie storage
const WHITELIST: &[&str] = &[".company.com", "api.company.com", "internal.company.net"];

/// Cookie data structure representing a cookie from the browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expiration_date: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct CachedCookies {
    pub cookies: Vec<CookieData>,
    pub fetched_at: SystemTime,
}

/// Thread-safe cookie storage with domain whitelist validation
pub struct CookieStore {
    storage: Arc<RwLock<HashMap<String, CachedCookies>>>,
    opt_in: Arc<RwLock<bool>>,
}

impl CookieStore {
    /// Create a new CookieStore instance
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            opt_in: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if a domain is whitelisted for cookie storage
    pub fn is_whitelisted(&self, domain: &str) -> bool {
        // Check exact matches first
        if WHITELIST.contains(&domain) {
            return true;
        }

        // Check domain suffixes (for wildcard entries like .company.com)
        for whitelist_entry in WHITELIST {
            if whitelist_entry.starts_with('.') {
                // This is a wildcard entry - check if domain ends with it
                if domain.ends_with(whitelist_entry) || domain == &whitelist_entry[1..] {
                    return true;
                }
            }
        }

        false
    }

    /// Store cookies for a specific domain
    pub async fn store_cookies(&self, domain: &str, cookies: Vec<CookieData>) {
        self.store_cookies_with_fetched_at(domain, cookies, SystemTime::now())
            .await;
    }

    /// Store cookies for a specific domain with explicit timestamp.
    /// This is mainly useful for deterministic tests.
    pub async fn store_cookies_with_fetched_at(
        &self,
        domain: &str,
        cookies: Vec<CookieData>,
        fetched_at: SystemTime,
    ) {
        let mut storage = self.storage.write().await;
        storage.insert(
            domain.to_string(),
            CachedCookies {
                cookies,
                fetched_at,
            },
        );
    }

    /// Retrieve cookies for a specific domain
    pub async fn get_cookies(&self, domain: &str) -> Option<Vec<CookieData>> {
        let storage = self.storage.read().await;
        storage.get(domain).map(|entry| entry.cookies.clone())
    }

    /// Retrieve cookies with fetched timestamp for a specific domain.
    pub async fn get_cached(&self, domain: &str) -> Option<CachedCookies> {
        let storage = self.storage.read().await;
        storage.get(domain).cloned()
    }

    pub async fn is_opted_in(&self) -> bool {
        *self.opt_in.read().await
    }

    pub async fn set_opt_in(&self, enabled: bool) {
        *self.opt_in.write().await = enabled;
    }
}

impl Default for CookieStore {
    fn default() -> Self {
        Self::new()
    }
}
