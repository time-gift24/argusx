use provider::Dialect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
    #[serde(rename = "zai")]
    Zai,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai_compatible",
            Self::Zai => "zai",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "openai_compatible" => Some(Self::OpenAiCompatible),
            "zai" => Some(Self::Zai),
            _ => None,
        }
    }

    pub fn dialect(self) -> Dialect {
        match self {
            Self::OpenAiCompatible => Dialect::Openai,
            Self::Zai => Dialect::Zai,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfileSummary {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderProfileInput {
    pub id: Option<String>,
    pub provider_kind: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProviderProfileInput {
    pub provider_kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntimeConfig {
    pub provider_kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderProfileRecord {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key_ciphertext: Vec<u8>,
    pub api_key_nonce: Vec<u8>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ProviderProfileRecord {
    pub fn summary(&self) -> ProviderProfileSummary {
        ProviderProfileSummary {
            id: self.id.clone(),
            provider_kind: self.provider_kind,
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            is_default: self.is_default,
        }
    }
}
