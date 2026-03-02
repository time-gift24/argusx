use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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

/// Thread-safe cookie storage with domain whitelist validation
pub struct CookieStore {
    storage: Arc<RwLock<HashMap<String, Vec<CookieData>>>>,
}

impl CookieStore {
    /// Create a new CookieStore instance
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
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
        let mut storage = self.storage.write().await;
        storage.insert(domain.to_string(), cookies);
    }

    /// Retrieve cookies for a specific domain
    pub async fn get_cookies(&self, domain: &str) -> Option<Vec<CookieData>> {
        let storage = self.storage.read().await;
        storage.get(domain).cloned()
    }
}

impl Default for CookieStore {
    fn default() -> Self {
        Self::new()
    }
}
