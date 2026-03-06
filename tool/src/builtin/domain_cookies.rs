use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

const DEFAULT_GATEWAY_BASE_URL: &str = "http://127.0.0.1:3456";

#[derive(Clone)]
pub struct DomainCookiesTool {
    client: reqwest::Client,
    gateway_base_url: String,
}

impl DomainCookiesTool {
    pub fn from_env() -> Self {
        let base_url = std::env::var("COOKIE_GATEWAY_BASE_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_GATEWAY_BASE_URL.to_string());
        Self::new(base_url)
    }

    pub fn new(gateway_base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            gateway_base_url: gateway_base_url.into(),
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/api/cookies/fetch",
            self.gateway_base_url.trim_end_matches('/')
        )
    }
}

#[derive(Debug, Deserialize)]
struct DomainCookieArgs {
    domain: String,
    refresh_after_ms: u64,
}

#[async_trait]
impl Tool for DomainCookiesTool {
    fn name(&self) -> &str {
        "get_domain_cookies"
    }

    fn description(&self) -> &str {
        "Fetch cookies for a domain from cookie-gateway, refreshing via browser extension when cache is stale"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Target domain, e.g. github.com"
                    },
                    "refresh_after_ms": {
                        "type": "integer",
                        "description": "Refresh threshold in milliseconds. 0 means force refresh.",
                        "minimum": 0
                    }
                },
                "required": ["domain", "refresh_after_ms"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let parsed: DomainCookieArgs = serde_json::from_value(args)
            .map_err(|err| ToolError::InvalidArgs(format!("invalid payload: {err}")))?;

        let domain = parsed.domain.trim();
        if domain.is_empty() {
            return Err(ToolError::InvalidArgs("domain is required".to_string()));
        }

        let payload = json!({
            "domain": domain,
            "refresh_after_ms": parsed.refresh_after_ms,
        });

        let response = self
            .client
            .post(self.endpoint())
            .json(&payload)
            .send()
            .await
            .map_err(|err| ToolError::ExecutionFailed(format!("gateway request failed: {err}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read error body>".to_string());
            return Err(ToolError::ExecutionFailed(format!(
                "gateway returned {status}: {body}"
            )));
        }

        let output = response.json::<serde_json::Value>().await.map_err(|err| {
            ToolError::ExecutionFailed(format!("invalid gateway response: {err}"))
        })?;

        Ok(ToolResult::ok(output))
    }
}
