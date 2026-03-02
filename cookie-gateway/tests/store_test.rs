use cookie_gateway::store::{CookieData, CookieStore};

#[test]
fn test_whitelist() {
    let store = CookieStore::new();
    assert!(store.is_whitelisted("api.company.com"));
    assert!(store.is_whitelisted("sub.company.com"));
    assert!(!store.is_whitelisted("google.com"));
}

#[tokio::test]
async fn test_store_and_retrieve_cookies() {
    let store = CookieStore::new();

    let cookies = vec![CookieData {
        name: "session".to_string(),
        value: "abc123".to_string(),
        domain: "api.company.com".to_string(),
        path: "/".to_string(),
        secure: true,
        http_only: true,
        expiration_date: Some(1234567890.0),
    }];

    // Store cookies
    store.store_cookies("api.company.com", cookies.clone()).await;

    // Retrieve cookies
    let retrieved = store.get_cookies("api.company.com").await;
    assert!(retrieved.is_some());

    let retrieved_cookies = retrieved.unwrap();
    assert_eq!(retrieved_cookies.len(), 1);
    assert_eq!(retrieved_cookies[0].name, "session");
    assert_eq!(retrieved_cookies[0].value, "abc123");
}

#[tokio::test]
async fn test_get_nonexistent_cookies() {
    let store = CookieStore::new();

    let result = store.get_cookies("nonexistent.com").await;
    assert!(result.is_none());
}

#[test]
fn test_whitelist_edge_cases() {
    let store = CookieStore::new();

    // Test exact matches
    assert!(store.is_whitelisted("api.company.com"));
    assert!(store.is_whitelisted("internal.company.net"));

    // Test subdomain matching with .company.com
    assert!(store.is_whitelisted("sub.company.com"));
    assert!(store.is_whitelisted("deep.sub.company.com"));
    assert!(store.is_whitelisted("company.com")); // Should match .company.com

    // Test non-whitelisted domains
    assert!(!store.is_whitelisted("google.com"));
    assert!(!store.is_whitelisted("company.org"));
    assert!(!store.is_whitelisted("notcompany.com"));
}
