use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Bigmodel,
    Openai,
    Anthropic,
}

impl ProviderId {
    pub fn as_adapter_id(&self) -> &'static str {
        match self {
            Self::Bigmodel => "bigmodel",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HeaderPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRuntimeConfig {
    pub api_key: String,
    pub base_url: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub headers: Vec<HeaderPair>,
}

impl ProviderRuntimeConfig {
    pub fn is_available(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && !self.models.is_empty()
    }

    pub fn header_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for pair in &self.headers {
            let key = pair.key.trim();
            if key.is_empty() {
                continue;
            }
            map.insert(key.to_string(), pair.value.clone());
        }
        map
    }

    pub fn normalize(&mut self) {
        self.api_key = self.api_key.trim().to_string();
        self.base_url = self.base_url.trim().trim_end_matches('/').to_string();

        let mut seen_models = HashSet::new();
        self.models = self
            .models
            .iter()
            .map(|m| m.trim())
            .filter(|m| !m.is_empty())
            .filter(|m| seen_models.insert((*m).to_string()))
            .map(ToString::to_string)
            .collect();

        let mut seen_headers = HashSet::new();
        let mut normalized_headers = Vec::new();
        // Keep the last value for duplicate keys.
        for pair in self.headers.iter().rev() {
            let key = pair.key.trim();
            if key.is_empty() {
                continue;
            }
            if !seen_headers.insert(key.to_string()) {
                continue;
            }
            normalized_headers.push(HeaderPair {
                key: key.to_string(),
                value: pair.value.clone(),
            });
        }
        normalized_headers.reverse();
        self.headers = normalized_headers;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigs {
    #[serde(default)]
    pub bigmodel: ProviderRuntimeConfig,
    #[serde(default)]
    pub openai: ProviderRuntimeConfig,
    #[serde(default)]
    pub anthropic: ProviderRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmRuntimeConfig {
    pub default_provider: Option<ProviderId>,
    #[serde(default)]
    pub providers: ProviderConfigs,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModel {
    pub provider: ProviderId,
    pub model: String,
}

impl LlmRuntimeConfig {
    pub fn provider(&self, provider: &ProviderId) -> &ProviderRuntimeConfig {
        match provider {
            ProviderId::Bigmodel => &self.providers.bigmodel,
            ProviderId::Openai => &self.providers.openai,
            ProviderId::Anthropic => &self.providers.anthropic,
        }
    }

    pub fn configured_providers(&self) -> Vec<ProviderId> {
        [
            ProviderId::Bigmodel,
            ProviderId::Openai,
            ProviderId::Anthropic,
        ]
        .into_iter()
        .filter(|provider| self.provider(provider).is_available())
        .collect()
    }
}

pub fn normalize_runtime_config(mut cfg: LlmRuntimeConfig) -> LlmRuntimeConfig {
    cfg.providers.bigmodel.normalize();
    cfg.providers.openai.normalize();
    cfg.providers.anthropic.normalize();

    let available = cfg.configured_providers();
    let default_invalid = cfg
        .default_provider
        .as_ref()
        .map(|provider| !cfg.provider(provider).is_available())
        .unwrap_or(true);

    if default_invalid {
        cfg.default_provider = available.into_iter().next();
    }

    cfg
}

pub fn list_available_models(cfg: &LlmRuntimeConfig) -> Vec<AvailableModel> {
    let mut models = Vec::new();
    for provider in [
        ProviderId::Bigmodel,
        ProviderId::Openai,
        ProviderId::Anthropic,
    ] {
        let provider_cfg = cfg.provider(&provider);
        if !provider_cfg.is_available() {
            continue;
        }
        for model in &provider_cfg.models {
            models.push(AvailableModel {
                provider: provider.clone(),
                model: model.clone(),
            });
        }
    }
    models
}

pub fn validate_turn_selection(
    cfg: &LlmRuntimeConfig,
    provider: &ProviderId,
    model: &str,
) -> Result<(), String> {
    let provider_cfg = cfg.provider(provider);
    if !provider_cfg.is_available() {
        return Err(format!(
            "provider '{}' is not configured",
            provider.as_adapter_id()
        ));
    }

    if !provider_cfg.models.iter().any(|m| m == model) {
        return Err(format!(
            "model '{}' is not enabled for provider '{}'",
            model,
            provider.as_adapter_id()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_available_models_work() {
        let cfg = LlmRuntimeConfig {
            default_provider: Some(ProviderId::Bigmodel),
            providers: ProviderConfigs {
                bigmodel: ProviderRuntimeConfig {
                    api_key: "".into(),
                    base_url: "https://bigmodel.provider.test/v1".into(),
                    models: vec!["glm-5".into()],
                    headers: vec![],
                },
                openai: ProviderRuntimeConfig {
                    api_key: " sk ".into(),
                    base_url: " https://openai.provider.test/v1/ ".into(),
                    models: vec![" gpt-4o ".into(), "gpt-4o".into()],
                    headers: vec![
                        HeaderPair {
                            key: " X-Test ".into(),
                            value: "1".into(),
                        },
                        HeaderPair {
                            key: "X-Test".into(),
                            value: "2".into(),
                        },
                    ],
                },
                anthropic: ProviderRuntimeConfig::default(),
            },
        };

        let normalized = normalize_runtime_config(cfg);
        assert_eq!(normalized.default_provider, Some(ProviderId::Openai));
        assert_eq!(
            normalized.providers.openai.base_url,
            "https://openai.provider.test/v1"
        );
        assert_eq!(normalized.providers.openai.models, vec!["gpt-4o"]);
        assert_eq!(normalized.providers.openai.headers.len(), 1);
        assert_eq!(normalized.providers.openai.headers[0].key, "X-Test");
        assert_eq!(normalized.providers.openai.headers[0].value, "2");

        let models = list_available_models(&normalized);
        assert_eq!(models.len(), 1);
        assert!(matches!(models[0].provider, ProviderId::Openai));
        assert_eq!(models[0].model, "gpt-4o");
    }

    #[test]
    fn validate_turn_selection_checks_provider_and_model() {
        let cfg = normalize_runtime_config(LlmRuntimeConfig {
            default_provider: Some(ProviderId::Openai),
            providers: ProviderConfigs {
                openai: ProviderRuntimeConfig {
                    api_key: "sk".into(),
                    base_url: "https://openai.provider.test/v1".into(),
                    models: vec!["gpt-4o".into()],
                    headers: vec![],
                },
                ..ProviderConfigs::default()
            },
        });

        let missing_provider = validate_turn_selection(&cfg, &ProviderId::Anthropic, "claude");
        assert!(missing_provider.is_err());

        let missing_model = validate_turn_selection(&cfg, &ProviderId::Openai, "gpt-5");
        assert!(missing_model.is_err());

        let ok = validate_turn_selection(&cfg, &ProviderId::Openai, "gpt-4o");
        assert!(ok.is_ok());
    }
}
