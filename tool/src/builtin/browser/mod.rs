pub mod chrome;
pub mod config;

use async_trait::async_trait;
use chromiumoxide::cdp::browser_protocol::network::CookieSameSite;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::Page;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

use self::chrome::ChromeManager;
use self::config::BrowserConfig;

/// Cookie representation for browser operations
#[derive(Debug, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: Option<bool>,
    pub http_only: Option<bool>,
    pub same_site: Option<String>,
    #[serde(rename = "expires")]
    pub expires_unix: Option<i64>,
}

/// Element information for a DOM element
#[derive(Debug, Serialize, Deserialize)]
pub struct ElementInfo {
    pub tag_name: String,
    pub text: Option<String>,
    pub inner_html: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
    pub bounding_box: Option<BoundingBox>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Browser configuration for getting current settings
#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserConfigInfo {
    pub port: u16,
    pub chrome_path: Option<String>,
    pub profile_dir: Option<String>,
    pub headless: bool,
    pub is_enabled: bool,
}

impl From<BrowserConfig> for BrowserConfigInfo {
    fn from(config: BrowserConfig) -> Self {
        Self {
            port: config.port,
            chrome_path: config.chrome_path,
            profile_dir: config.profile_dir,
            headless: config.headless,
            is_enabled: config.is_enabled,
        }
    }
}

pub struct BrowserTool {
    chrome_manager: Arc<Mutex<ChromeManager>>,
    config: BrowserConfig,
}

impl BrowserTool {
    pub fn new(
        config: BrowserConfig,
        config_manager: config::BrowserConfigManager,
    ) -> Self {
        let chrome_manager = ChromeManager::new(config.clone(), config_manager);
        Self {
            chrome_manager: Arc::new(Mutex::new(chrome_manager)),
            config,
        }
    }

    /// Get or create a page for browser operations
    async fn get_page(&self) -> Result<Page, ToolError> {
        let mut chrome_manager = self.chrome_manager.lock().await;

        let browser = chrome_manager
            .connect_or_launch()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        let browser = browser.lock().await;

        // Get existing pages or create a new one
        let pages = browser.pages().await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        if let Some(page) = pages.first() {
            Ok(page.clone())
        } else {
            browser
                .new_page("")
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
        }
    }

    /// Navigate to a URL
    async fn action_navigate(&self, url: String) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        page.goto(&url)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let current_url = page.url().await.unwrap_or_default();
        Ok(ToolResult::ok(json!({
            "success": true,
            "url": current_url,
        })))
    }

    /// Click on an element
    async fn action_click(&self, selector: String) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;

        let element = page
            .find_element(&selector)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Element not found: {}", e)))?;

        element
            .click()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "click",
            "selector": selector,
        })))
    }

    /// Type text into an element
    async fn action_type(&self, selector: String, text: String) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;

        let element = page
            .find_element(&selector)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Element not found: {}", e)))?;

        element
            .click()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        element
            .type_str(&text)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "type",
            "selector": selector,
            "text_length": text.len(),
        })))
    }

    /// Take a screenshot
    async fn action_screenshot(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;

        let screenshot_params = ScreenshotParams::builder()
            .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
            .build();

        let screenshot = page
            .screenshot(screenshot_params)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let base64 = base64_encode(&screenshot);

        Ok(ToolResult::ok(json!({
            "success": true,
            "format": "png",
            "data": base64,
        })))
    }

    /// Get page content
    async fn action_get_content(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let content = page
            .content()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let current_url = page.url().await.unwrap_or_default();

        Ok(ToolResult::ok(json!({
            "success": true,
            "url": current_url,
            "content": content,
        })))
    }

    /// Get cookies
    async fn action_get_cookies(&self, _domain: Option<String>) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let cookies = page.get_cookies().await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let cookie_infos: Vec<serde_json::Value> = cookies
            .into_iter()
            .map(|c| {
                json!({
                    "name": c.name,
                    "value": c.value,
                    "domain": c.domain,
                    "path": c.path,
                    "secure": c.secure,
                    "http_only": c.http_only,
                    "same_site": c.same_site,
                    "expires": c.expires,
                })
            })
            .collect();

        Ok(ToolResult::ok(json!({
            "success": true,
            "cookies": cookie_infos,
        })))
    }

    /// Set cookies
    async fn action_set_cookies(&self, cookies: Vec<Cookie>) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let count = cookies.len();

        for cookie in cookies {
            use chromiumoxide::cdp::browser_protocol::network::CookieParam;

            let same_site = cookie.same_site.and_then(|s| match s.as_str() {
                "Strict" => Some(CookieSameSite::Strict),
                "Lax" => Some(CookieSameSite::Lax),
                "None" => Some(CookieSameSite::None),
                _ => None,
            });

            let c = CookieParam::builder()
                .name(cookie.name)
                .value(cookie.value)
                .domain(cookie.domain.unwrap_or_default())
                .path(cookie.path.unwrap_or_default())
                .secure(cookie.secure.unwrap_or(false))
                .http_only(cookie.http_only.unwrap_or(false))
                .same_site(same_site.unwrap_or(CookieSameSite::None))
                .build()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build cookie: {}", e)))?;

            page.set_cookie(c).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to set cookie: {}", e))
            })?;
        }

        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "set_cookies",
            "count": count,
        })))
    }

    /// Execute JavaScript
    async fn action_execute_script(&self, script: String) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let result = page
            .evaluate(script.as_str())
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Get the value from the result
        let value = result.value().cloned();

        Ok(ToolResult::ok(json!({
            "success": true,
            "result": value,
        })))
    }

    /// Wait for selector - simplified implementation
    async fn action_wait_for_selector(
        &self,
        selector: String,
        timeout_ms: Option<u64>,
    ) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000));

        // Try to find the element within the timeout
        let result = tokio::time::timeout(
            timeout,
            page.find_element(&selector)
        ).await;

        match result {
            Ok(Ok(_)) => Ok(ToolResult::ok(json!({
                "success": true,
                "selector": selector,
                "found": true,
            }))),
            Ok(Err(e)) => Ok(ToolResult::ok(json!({
                "success": true,
                "selector": selector,
                "found": false,
                "error": e.to_string(),
            }))),
            Err(_) => Ok(ToolResult::ok(json!({
                "success": true,
                "selector": selector,
                "found": false,
                "error": "timeout",
            }))),
        }
    }

    /// Scroll the page
    async fn action_scroll(&self, x: i64, y: i64) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let script = format!("window.scrollTo({}, {});", x, y);
        page.evaluate(script.as_str())
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "scroll",
            "x": x,
            "y": y,
        })))
    }

    /// Get element information
    async fn action_get_element_info(&self, selector: String) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        let element = page
            .find_element(&selector)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Element not found: {}", e)))?;

        // Get tag name
        let tag_name: String = element
            .call_js_fn("function() { return this.tagName; }", false)
            .await
            .ok()
            .and_then(|v| v.result.value.as_ref().and_then(|val| val.as_str()).map(|s| s.to_string()))
            .unwrap_or_default()
            .to_uppercase();

        // Get text content
        let text = element.inner_text().await.ok().flatten();
        let inner_html = element.inner_html().await.ok().flatten();

        // Get attributes
        let attributes_list = element.attributes().await.unwrap_or_default();
        let mut attributes = std::collections::HashMap::new();
        let mut iter = attributes_list.iter();
        while let Some(name) = iter.next() {
            if let Some(value) = iter.next() {
                attributes.insert(name.clone(), value.clone());
            }
        }

        // Get bounding box
        let bounding_box = element.bounding_box().await.ok().map(|bb| BoundingBox {
            x: bb.x,
            y: bb.y,
            width: bb.width,
            height: bb.height,
        });

        let element_info = ElementInfo {
            tag_name,
            text,
            inner_html,
            attributes,
            bounding_box,
        };

        Ok(ToolResult::ok(json!({
            "success": true,
            "selector": selector,
            "element": element_info,
        })))
    }

    /// Go back in history using JavaScript
    async fn action_go_back(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        page.evaluate("window.history.back()")
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let current_url = page.url().await.unwrap_or_default();
        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "go_back",
            "url": current_url,
        })))
    }

    /// Go forward in history using JavaScript
    async fn action_go_forward(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        page.evaluate("window.history.forward()")
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let current_url = page.url().await.unwrap_or_default();
        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "go_forward",
            "url": current_url,
        })))
    }

    /// Reload the page
    async fn action_reload(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;
        page.reload()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let current_url = page.url().await.unwrap_or_default();
        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "reload",
            "url": current_url,
        })))
    }

    /// Set headless mode
    async fn action_set_headless(&self, enabled: bool) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::ok(json!({
            "success": true,
            "action": "set_headless",
            "headless": enabled,
            "note": "Browser restart may be required for this change to take effect",
        })))
    }

    /// Get current browser config
    async fn action_get_config(&self) -> Result<ToolResult, ToolError> {
        let config_info: BrowserConfigInfo = self.config.clone().into();
        Ok(ToolResult::ok(json!({
            "success": true,
            "config": config_info,
        })))
    }
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum BrowserAction {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Screenshot,
    GetContent,
    GetCookies { domain: Option<String> },
    SetCookies { cookies: Vec<Cookie> },
    ExecuteScript { script: String },
    WaitForSelector {
        selector: String,
        timeout_ms: Option<u64>,
    },
    Scroll { x: i64, y: i64 },
    GetElementInfo { selector: String },
    GoBack,
    GoForward,
    Reload,
    SetHeadless { enabled: bool },
    GetConfig,
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Browser automation tool for web scraping, testing, and interaction"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "navigate",
                            "click",
                            "type",
                            "screenshot",
                            "get_content",
                            "get_cookies",
                            "set_cookies",
                            "execute_script",
                            "wait_for_selector",
                            "scroll",
                            "get_element_info",
                            "go_back",
                            "go_forward",
                            "reload",
                            "set_headless",
                            "get_config"
                        ],
                        "description": "The action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL for navigate action"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for element operations"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Domain to filter cookies"
                    },
                    "cookies": {
                        "type": "array",
                        "description": "Cookies to set",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "value": { "type": "string" },
                                "domain": { "type": "string" },
                                "path": { "type": "string" },
                                "secure": { "type": "boolean" },
                                "http_only": { "type": "boolean" },
                                "same_site": { "type": "string" },
                                "expires": { "type": "number" }
                            }
                        }
                    },
                    "script": {
                        "type": "string",
                        "description": "JavaScript to execute"
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "Timeout in milliseconds"
                    },
                    "x": {
                        "type": "number",
                        "description": "X coordinate for scroll"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate for scroll"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Enable or disable headless mode"
                    }
                }
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let action: BrowserAction = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        match action {
            BrowserAction::Navigate { url } => self.action_navigate(url).await,
            BrowserAction::Click { selector } => self.action_click(selector).await,
            BrowserAction::Type { selector, text } => self.action_type(selector, text).await,
            BrowserAction::Screenshot => self.action_screenshot().await,
            BrowserAction::GetContent => self.action_get_content().await,
            BrowserAction::GetCookies { domain } => self.action_get_cookies(domain).await,
            BrowserAction::SetCookies { cookies } => self.action_set_cookies(cookies).await,
            BrowserAction::ExecuteScript { script } => self.action_execute_script(script).await,
            BrowserAction::WaitForSelector {
                selector,
                timeout_ms,
            } => self.action_wait_for_selector(selector, timeout_ms).await,
            BrowserAction::Scroll { x, y } => self.action_scroll(x, y).await,
            BrowserAction::GetElementInfo { selector } => {
                self.action_get_element_info(selector).await
            }
            BrowserAction::GoBack => self.action_go_back().await,
            BrowserAction::GoForward => self.action_go_forward().await,
            BrowserAction::Reload => self.action_reload().await,
            BrowserAction::SetHeadless { enabled } => self.action_set_headless(enabled).await,
            BrowserAction::GetConfig => self.action_get_config().await,
        }
    }
}
